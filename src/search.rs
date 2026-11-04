//! Single-threaded PVS alpha-beta search over `SANTNNUE03` for WebAssembly (`wasm32-unknown-unknown`).
//!
//! Includes threat quiescence, singular extensions, internal iterative reduction, null-move
//! pruning with Pantheon start-of-turn tag refresh, commutative dome symmetry breaking
//! (`Heracles` / `Polyphemus`), lossless intra-turn walk-cycle & 1-step equivalence pruning
//! (`Hermes`, `Triton`, `Atalanta`), and Multi-PV root analysis.

use crate::compact::*;
use crate::gods::{
    decode_action, ActionKind, GOD_ARES, GOD_BIA, GOD_HARPIES, GOD_HERMES, GOD_IRIS, GOD_PAN, GOD_POLYPHEMUS,
    GOD_SELENE, GOD_URANIA, PASS_ACTION, STAGE_CHAIN, STAGE_DEMETER, STAGE_END, STAGE_HEPHAESTUS, STAGE_HERACLES,
    STAGE_HERMES, STAGE_HESTIA, STAGE_NONE, STAGE_POSEIDON, STAGE_SCYLLA, TAG_FEMALE, WRAP_NEIGHBORS,
};
use crate::nnue::{Nnue, NnueAccCache};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub const MATE: i32 = 30_000;
pub const MATE_BOUND: i32 = MATE - 1_000;
pub const INF: i32 = 32_000;
pub const EVAL_SCALE: f32 = 10_000.0;
const MAX_PLY: usize = 96;
const EVAL_CACHE_SIZE: usize = 1 << 16;
const NO_MOVE: i32 = -1;

pub const TT_EXACT: u8 = 1;
pub const TT_LOWER: u8 = 2;
pub const TT_UPPER: u8 = 3;

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn js_now() -> f64;
}

#[inline(always)]
pub fn current_time_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js_now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    }
}

// ---------------------------------------------------------------------------
// Transposition table
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct TtEntry {
    pub score: i32,
    pub action: i32,
    pub depth: u8,
    pub flag: u8,
}

#[repr(C, align(16))]
struct TtSlot {
    key: AtomicU64,
    data: AtomicU64,
}

pub struct Tt {
    slots: Vec<TtSlot>,
    mask: usize,
}

impl Tt {
    pub fn new(power: usize) -> Self {
        let n = 1usize << power;
        Self {
            slots: (0..n).map(|_| TtSlot { key: AtomicU64::new(0), data: AtomicU64::new(0) }).collect(),
            mask: n - 1,
        }
    }

    pub fn clear(&self) {
        for e in &self.slots {
            e.key.store(0, Ordering::Relaxed);
            e.data.store(0, Ordering::Relaxed);
        }
    }

    #[inline(always)]
    fn to_tt(score: i32, ply: usize) -> i32 {
        if score >= MATE_BOUND {
            score + ply as i32
        } else if score <= -MATE_BOUND {
            score - ply as i32
        } else {
            score
        }
    }

    #[inline(always)]
    fn from_tt(score: i32, ply: usize) -> i32 {
        if score >= MATE_BOUND {
            score - ply as i32
        } else if score <= -MATE_BOUND {
            score + ply as i32
        } else {
            score
        }
    }

    #[inline]
    pub fn probe(&self, hash: u64, ply: usize) -> Option<TtEntry> {
        let e = &self.slots[hash as usize & self.mask];
        let d = e.data.load(Ordering::Relaxed);
        if d == 0 || e.key.load(Ordering::Relaxed) ^ d != hash {
            return None;
        }
        Some(TtEntry {
            score: Self::from_tt(d as u16 as i16 as i32, ply),
            action: ((d >> 16) & 0x1FF) as i32 - 1,
            depth: (d >> 25) as u8,
            flag: ((d >> 33) & 0x3) as u8,
        })
    }

    #[inline]
    pub fn store(&self, hash: u64, score: i32, action: i32, depth: u8, flag: u8, ply: usize) {
        let e = &self.slots[hash as usize & self.mask];
        let old = e.data.load(Ordering::Relaxed);
        if old != 0 && e.key.load(Ordering::Relaxed) ^ old == hash && depth < (old >> 25) as u8 && flag != TT_EXACT {
            return;
        }
        let s = Self::to_tt(score, ply) as i16 as u16 as u64;
        let d = s | (((action + 1) as u64 & 0x1FF) << 16) | ((depth as u64) << 25) | ((flag as u64) << 33);
        e.key.store(hash ^ d, Ordering::Relaxed);
        e.data.store(d, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Search Parameters & Results
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchParams {
    pub lmr_base: f32,
    pub lmr_div: f32,
    pub rfp_max_depth: i32,
    pub rfp_margin: i32,
    pub nmp_base: i32,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self { lmr_base: 1.25, lmr_div: 1.4, rfp_max_depth: 7, rfp_margin: 500, nmp_base: 4 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub time_ms: u64,
    pub max_depth: u8,
    pub max_nodes: u64,
}

#[derive(Clone, Debug)]
pub struct PvLine {
    pub action: i32,
    pub score: i32,
    pub depth: u8,
    pub pv: Vec<i32>,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub best_action: i32,
    pub score: i32,
    pub depth: u8,
    pub nodes: u64,
    pub elapsed_ms: f64,
    pub pv: Vec<i32>,
    pub lines: Vec<PvLine>,
}

pub struct SearchEngine {
    pub nnue: Arc<Nnue>,
    pub tt: Arc<Tt>,
    pub params: SearchParams,
    history: Box<[[i32; ACTION_SPACE]; 2]>,
    saved_history: HashMap<[u8; 2], Box<[[i32; ACTION_SPACE]; 2]>>,
    last_gods: [u8; 2],
}

impl SearchEngine {
    pub fn new(nnue: Arc<Nnue>, tt_power: usize) -> Self {
        Self {
            nnue,
            tt: Arc::new(Tt::new(tt_power)),
            params: SearchParams::default(),
            history: Box::new([[0; ACTION_SPACE]; 2]),
            saved_history: HashMap::new(),
            last_gods: [u8::MAX; 2],
        }
    }

    pub fn clear(&mut self) {
        self.tt.clear();
        *self.history = [[0; ACTION_SPACE]; 2];
        self.saved_history.clear();
        self.last_gods = [u8::MAX; 2];
    }

    pub fn static_eval(&self, s: &CompactSantoriniState) -> i32 {
        (self.nnue.evaluate(s, &mut NnueAccCache::default()).tanh() * EVAL_SCALE) as i32
    }

    pub fn search(&mut self, root: &CompactSantoriniState, limits: Limits) -> SearchResult {
        self.search_multipv(root, limits, 1)
    }

    pub fn search_multipv(&mut self, root: &CompactSantoriniState, limits: Limits, multi_pv: usize) -> SearchResult {
        let start = current_time_ms();
        if root.gods != self.last_gods {
            if self.last_gods != [u8::MAX; 2] {
                if self.saved_history.len() >= 32 && !self.saved_history.contains_key(&self.last_gods) {
                    self.saved_history.clear();
                }
                let blank = Box::new([[0; ACTION_SPACE]; 2]);
                self.saved_history.insert(self.last_gods, std::mem::replace(&mut self.history, blank));
            }
            if let Some(prev) = self.saved_history.remove(&root.gods) {
                self.history = prev;
            } else {
                *self.history = [[0; ACTION_SPACE]; 2];
            }
            self.last_gods = root.gods;
        }

        let mut legal = [0i32; MAX_ACTIONS];
        let n = root.write_valid_actions_into(&mut legal);
        assert!(n > 0, "search() on terminal state");
        if n == 1 {
            return SearchResult {
                best_action: legal[0],
                score: 0,
                depth: 0,
                nodes: 1,
                elapsed_ms: 0.0,
                pv: vec![legal[0]],
                lines: vec![PvLine { action: legal[0], score: 0, depth: 0, pv: vec![legal[0]] }],
            };
        }

        let k = multi_pv.clamp(1, 5).min(n);
        let deadline = start + limits.time_ms as f64;
        let mut worker = Worker::new(&self.nnue, &self.tt, deadline, limits.max_nodes, *self.history, self.params);
        let mpv_lines = if k == 1 {
            worker.iterate(root, limits.max_depth);
            Vec::new()
        } else {
            worker.iterate_multipv(root, limits.max_depth, k)
        };
        *self.history = worker.history;
        let elapsed = (current_time_ms() - start).max(0.0);

        if k == 1 {
            let pv = self.extract_pv(root, worker.best_move);
            let lines = vec![PvLine {
                action: worker.best_move,
                score: worker.best_score,
                depth: worker.completed_depth,
                pv: pv.clone(),
            }];
            return SearchResult {
                best_action: worker.best_move,
                score: worker.best_score,
                depth: worker.completed_depth,
                nodes: worker.nodes,
                elapsed_ms: elapsed,
                pv,
                lines,
            };
        }

        let lines: Vec<PvLine> = mpv_lines
            .into_iter()
            .map(|(action, score, depth)| PvLine { action, score, depth, pv: self.extract_pv(root, action) })
            .collect();
        let top = &lines[0];
        SearchResult {
            best_action: top.action,
            score: top.score,
            depth: top.depth,
            nodes: worker.nodes,
            elapsed_ms: elapsed,
            pv: top.pv.clone(),
            lines,
        }
    }

    fn extract_pv(&self, root: &CompactSantoriniState, first: i32) -> Vec<i32> {
        let mut pv = vec![first];
        let mut s = *root;
        s.apply_action(first);
        let mut buf = [0i32; MAX_ACTIONS];
        while pv.len() < 24 && !s.game_over {
            let Some(e) = self.tt.probe(s.state_hash(), 0) else { break };
            let n = s.write_valid_actions_into(&mut buf);
            if e.action < 0 || !buf[..n].contains(&e.action) {
                break;
            }
            pv.push(e.action);
            s.apply_action(e.action);
        }
        pv
    }
}

// ---------------------------------------------------------------------------
// Search Worker
// ---------------------------------------------------------------------------

struct Worker<'a> {
    nnue: &'a Nnue,
    tt: &'a Tt,
    deadline: f64,
    max_nodes: u64,
    params: SearchParams,
    cache: NnueAccCache,
    eval_cache: Box<[(u64, i32)]>,
    killers: [[i32; 2]; MAX_PLY],
    counter: Box<[[i32; ACTION_SPACE]; 2]>,
    history: [[i32; ACTION_SPACE]; 2],
    prev_move: [i32; MAX_PLY + 1],
    walk_key: [([u8; 2], u8); MAX_PLY + 1],
    walk_start: [usize; MAX_PLY + 1],
    walk_moved_to: [u8; MAX_PLY + 1],
    nodes: u64,
    aborted: bool,
    best_move: i32,
    best_score: i32,
    completed_depth: u8,
}

#[inline]
fn lmr(p: &SearchParams, depth: i32, idx: usize, is_pv: bool) -> i32 {
    if depth < 2 || idx < 2 {
        return 0;
    }
    let r = p.lmr_base + (depth as f32).ln() * (idx as f32).ln() / p.lmr_div;
    (r as i32 + !is_pv as i32 - 1).max(0)
}

impl<'a> Worker<'a> {
    fn new(
        nnue: &'a Nnue,
        tt: &'a Tt,
        deadline: f64,
        max_nodes: u64,
        history: [[i32; ACTION_SPACE]; 2],
        params: SearchParams,
    ) -> Self {
        Self {
            nnue,
            tt,
            deadline,
            max_nodes,
            params,
            cache: NnueAccCache::default(),
            eval_cache: vec![(0u64, 0i32); EVAL_CACHE_SIZE].into_boxed_slice(),
            killers: [[NO_MOVE; 2]; MAX_PLY],
            counter: Box::new([[NO_MOVE; ACTION_SPACE]; 2]),
            history,
            prev_move: [NO_MOVE; MAX_PLY + 1],
            walk_key: [([UNPLACED; 2], 0); MAX_PLY + 1],
            walk_start: [0; MAX_PLY + 1],
            walk_moved_to: [UNPLACED; MAX_PLY + 1],
            nodes: 0,
            aborted: false,
            best_move: NO_MOVE,
            best_score: 0,
            completed_depth: 0,
        }
    }

    #[inline]
    fn check_abort(&mut self) -> bool {
        if self.aborted {
            return true;
        }
        if self.nodes >= self.max_nodes {
            self.aborted = true;
            return true;
        }
        if self.nodes & 1023 == 0 && current_time_ms() >= self.deadline {
            self.aborted = true;
        }
        self.aborted
    }

    #[inline]
    fn eval(&mut self, s: &CompactSantoriniState, h: u64) -> i32 {
        let slot = &mut self.eval_cache[h as usize & (EVAL_CACHE_SIZE - 1)];
        if slot.0 == h && h != 0 {
            return slot.1;
        }
        let v = (self.nnue.evaluate(s, &mut self.cache).tanh() * EVAL_SCALE) as i32;
        let slot = &mut self.eval_cache[h as usize & (EVAL_CACHE_SIZE - 1)];
        *slot = (h, v);
        v
    }

    fn iterate(&mut self, root: &CompactSantoriniState, max_depth: u8) {
        let mut legal = [0i32; MAX_ACTIONS];
        let n = root.write_valid_actions_into(&mut legal);
        self.best_move = legal[0];
        if root.has_winning_move() {
            for &a in &legal[..n] {
                let mut c = *root;
                c.apply_action(a);
                if c.game_over && c.winner == root.current_player as i8 {
                    self.best_move = a;
                    self.best_score = MATE - 1;
                    self.completed_depth = 1;
                    return;
                }
            }
        }
        let mut score = 0;
        let mut depth = 1;
        while depth <= max_depth as i32 {
            let mut delta = 300;
            let (mut alpha, mut beta) = if depth >= 4 { (score - delta, score + delta) } else { (-INF, INF) };
            loop {
                let (v, mv) = self.root_search(root, depth, alpha, beta);
                if self.aborted {
                    return;
                }
                if v <= alpha {
                    beta = (alpha + beta) / 2;
                    alpha = (v - delta).max(-INF);
                } else if v >= beta {
                    beta = (v + delta).min(INF);
                    if mv != NO_MOVE {
                        self.best_move = mv;
                    }
                } else {
                    score = v;
                    self.best_move = mv;
                    self.best_score = v;
                    break;
                }
                delta += delta / 2;
            }
            self.completed_depth = depth as u8;
            if score.abs() >= MATE_BOUND && (MATE - score.abs()) <= depth {
                break;
            }
            depth += 1;
        }
    }

    fn iterate_multipv(&mut self, root: &CompactSantoriniState, max_depth: u8, k: usize) -> Vec<(i32, i32, u8)> {
        let mut moves = [0i32; MAX_ACTIONS];
        let mut scores = [0i32; MAX_ACTIONS];
        let n = root.write_valid_actions_into(&mut moves);
        let opp_threat = root.has_winning_move_for(1 - root.current_player as usize);
        self.score_moves(root, &moves[..n], &mut scores[..n], NO_MOVE, 0, opp_threat);
        let mut best_lines = Vec::with_capacity(k);
        for i in 0..k {
            pick_next(&mut moves[..n], &mut scores[..n], i);
            best_lines.push((moves[i], 0i32, 0u8));
        }
        self.best_move = best_lines[0].0;

        let mut depth = 1i32;
        while depth <= max_depth as i32 {
            let mut excluded = [NO_MOVE; 5];
            let mut cur_lines = Vec::with_capacity(k);
            for pv_idx in 0..k {
                let (pref_mv, prev_sc, _) = best_lines[pv_idx];
                let mut delta = 300;
                let (mut alpha, mut beta) = if depth >= 4 {
                    (prev_sc - delta, prev_sc + delta)
                } else {
                    (-INF, INF)
                };
                let pref = if depth > 1 { pref_mv } else { NO_MOVE };
                let (mut line_sc, mut line_mv) = (prev_sc, pref_mv);
                loop {
                    let (v, mv) = self.root_search_excluding(root, depth, alpha, beta, pref, &excluded[..pv_idx]);
                    if self.aborted {
                        break;
                    }
                    if v <= alpha {
                        beta = (alpha + beta) / 2;
                        alpha = (v - delta).max(-INF);
                    } else if v >= beta {
                        beta = (v + delta).min(INF);
                        if mv != NO_MOVE {
                            line_mv = mv;
                        }
                    } else {
                        line_sc = v;
                        line_mv = mv;
                        break;
                    }
                    delta += delta / 2;
                }
                if self.aborted || line_mv == NO_MOVE {
                    break;
                }
                excluded[pv_idx] = line_mv;
                cur_lines.push((line_mv, line_sc, depth as u8));
            }
            if self.aborted || cur_lines.len() < k {
                if !cur_lines.is_empty() {
                    for &(mv, sc, _) in &best_lines {
                        if cur_lines.len() >= k {
                            break;
                        }
                        if !cur_lines.iter().any(|x| x.0 == mv) {
                            cur_lines.push((mv, sc, self.completed_depth));
                        }
                    }
                    cur_lines.sort_by_key(|b| std::cmp::Reverse(b.1));
                    best_lines = cur_lines;
                    self.best_move = best_lines[0].0;
                    self.best_score = best_lines[0].1;
                }
                break;
            }
            cur_lines.sort_by_key(|b| std::cmp::Reverse(b.1));
            best_lines = cur_lines;
            self.best_move = best_lines[0].0;
            self.best_score = best_lines[0].1;
            self.completed_depth = depth as u8;
            depth += 1;
        }
        best_lines
    }

    fn root_search(&mut self, s: &CompactSantoriniState, depth: i32, alpha: i32, beta: i32) -> (i32, i32) {
        let tt_move = if self.completed_depth > 0 { self.best_move } else { NO_MOVE };
        self.root_search_excluding(s, depth, alpha, beta, tt_move, &[])
    }

    fn root_search_excluding(
        &mut self,
        s: &CompactSantoriniState,
        depth: i32,
        mut alpha: i32,
        beta: i32,
        tt_move: i32,
        excluded: &[i32],
    ) -> (i32, i32) {
        let mut moves = [0i32; MAX_ACTIONS];
        let mut scores = [0i32; MAX_ACTIONS];
        let n = s.write_valid_actions_into(&mut moves);
        let opp_threat = s.has_winning_move_for(1 - s.current_player as usize);
        self.score_moves(s, &moves[..n], &mut scores[..n], tt_move, 0, opp_threat);
        let mut best = -INF;
        let mut best_move = NO_MOVE;
        let root_hash = s.state_hash();
        let rp = s.current_player as usize;
        self.walk_key[0] = (s.workers[rp], (s.turn_flags >> rp) & 1);
        self.walk_start[0] = 0;
        self.walk_moved_to[0] = UNPLACED;
        let mut searched = 0usize;
        for i in 0..n {
            pick_next(&mut moves[..n], &mut scores[..n], i);
            let m = moves[i];
            if excluded.contains(&m) {
                continue;
            }
            let mut c = *s;
            c.apply_action(m);
            let ch = s.child_hash(root_hash, m, &c);
            let same_side = c.current_player == s.current_player;
            let next_depth = if same_side { depth } else { depth - 1 };
            self.prev_move[1] = m;
            self.walk_start[1] = if same_side { 0 } else { 1 };
            self.walk_moved_to[1] = match (s.stage, s.gods[rp], s.gods[1 - rp], decode_action(m)) {
                (STAGE_NONE, GOD_HERMES, og, ActionKind::MoveOnly { w, m: dir }) if og != GOD_HARPIES => {
                    NEIGHBORS[s.workers[rp][w] as usize][dir]
                }
                _ => UNPLACED,
            };
            let v = if searched == 0 {
                self.child(s, &c, ch, next_depth, 1, alpha, beta, true)
            } else {
                let r = if same_side {
                    lmr(&self.params, depth, searched.max(2), true).max(1).min(next_depth).max(0)
                } else if searched >= 3 && !is_climb(s, m) {
                    lmr(&self.params, depth, searched, true).min(next_depth - 1).max(0)
                } else {
                    0
                };
                let mut v = self.child(s, &c, ch, next_depth - r, 1, alpha, alpha + 1, false);
                if v > alpha && r > 0 && !self.aborted {
                    v = self.child(s, &c, ch, next_depth, 1, alpha, alpha + 1, false);
                }
                if v > alpha && v < beta && !self.aborted {
                    v = self.child(s, &c, ch, next_depth, 1, alpha, beta, true);
                }
                v
            };
            searched += 1;
            if self.aborted {
                break;
            }
            if v > best {
                best = v;
                best_move = m;
                if v > alpha {
                    alpha = v;
                    if v >= beta {
                        break;
                    }
                }
            }
        }
        if !self.aborted && best_move != NO_MOVE && excluded.is_empty() {
            let flag = if best >= beta { TT_LOWER } else { TT_EXACT };
            self.tt.store(root_hash, best, best_move, depth as u8, flag, 0);
        }
        (best, best_move)
    }

    fn score_moves(
        &self,
        s: &CompactSantoriniState,
        moves: &[i32],
        scores: &mut [i32],
        tt_move: i32,
        ply: usize,
        opp_threat: bool,
    ) {
        let p = s.current_player as usize;
        let counter = if self.prev_move[ply] >= 0 { self.counter[p][self.prev_move[ply] as usize] } else { NO_MOVE };
        let killers = if ply < MAX_PLY { self.killers[ply] } else { [NO_MOVE; 2] };
        for (m, sc) in moves.iter().zip(scores.iter_mut()) {
            let a = *m;
            *sc = if a == tt_move {
                1 << 28
            } else {
                match decode_action(a) {
                    ActionKind::Place(q) => {
                        let (r, c) = (q as i32 / 5 - 2, q as i32 % 5 - 2);
                        -(r.abs().max(c.abs()))
                    }
                    ActionKind::Turn { w, m, b, dome } => {
                        let from = s.workers[p][w] as usize;
                        let to = step_dest(s, p, from, m);
                        if to == OFF_BOARD {
                            self.history[p][a as usize] / 8
                        } else {
                            let bsq = build_dest(s, p, w, to as usize, b, dome);
                            if bsq == OFF_BOARD {
                                self.history[p][a as usize] / 8
                            } else {
                                let (hf, ht, hb) = (
                                    s.height[from] as i32,
                                    s.height[to as usize] as i32,
                                    s.height[bsq as usize] as i32,
                                );
                                let mut v = self.history[p][a as usize] / 8;
                                if !s.is_mortal_game() && s.gods[p] == GOD_PAN && hf >= ht + 2 {
                                    v += 8_000;
                                } else {
                                    v += 2_000 * (ht - hf);
                                }
                                let stops = opp_threat && blocks_threat(s, 1 - p, to as usize, bsq as usize);
                                if stops {
                                    v += 1 << 26;
                                }
                                if !s.is_mortal_game() {
                                    if s.workers[1 - p].contains(&to) {
                                        v += 400 + 1_500 * ht;
                                    }
                                    if s.gods[p] == GOD_BIA {
                                        let next = NEIGHBORS[to as usize][m];
                                        if next != OFF_BOARD && s.workers[1 - p].contains(&next) {
                                            v += 6_000;
                                        }
                                    }
                                    if dome {
                                        if hb == 2
                                            && s.workers[1 - p]
                                                .iter()
                                                .any(|&o| o != UNPLACED && NEIGHBORS[o as usize].contains(&bsq))
                                        {
                                            v += 1_200;
                                        } else if hb <= 1 && !stops {
                                            v -= 400;
                                        }
                                    }
                                }
                                if !dome && hb == ht && hb + 1 < DOME as i32 {
                                    v += 500;
                                }
                                if a == killers[0] || a == killers[1] {
                                    v += 1 << 22;
                                } else if a == counter {
                                    v += 1 << 21;
                                }
                                v
                            }
                        }
                    }
                    ActionKind::MoveOnly { w, m } => {
                        let from = s.workers[p][w] as usize;
                        let to = step_dest(s, p, from, m);
                        let mut v = self.history[p][a as usize] / 8;
                        if to != OFF_BOARD {
                            v += 2_000 * (s.height[to as usize] as i32 - s.height[from] as i32);
                            if !s.is_mortal_game() && s.workers[1 - p].contains(&to) {
                                v += 400 + 1_500 * s.height[to as usize] as i32;
                            }
                        }
                        v
                    }
                    ActionKind::Zeus { w, m } => {
                        let from = s.workers[p][w] as usize;
                        let to = NEIGHBORS[from][m];
                        let climb = if to == OFF_BOARD {
                            0
                        } else {
                            s.height[to as usize] as i32 + 1 - s.height[from] as i32
                        };
                        self.history[p][a as usize] / 8 + 2_000 * climb
                    }
                    ActionKind::Pass => self.history[p][a as usize] / 8 + 100,
                    ActionKind::BuildOnly { w, d } => {
                        let mut v = self.history[p][a as usize] / 8;
                        let wsq = s.workers[p][w];
                        if wsq != UNPLACED {
                            let bsq = build_dest(s, p, w, wsq as usize, d, false);
                            if bsq != OFF_BOARD {
                                if opp_threat && blocks_threat(s, 1 - p, wsq as usize, bsq as usize) {
                                    v += 1 << 26;
                                }
                                let (hw, hb) = (s.height[wsq as usize], s.height[bsq as usize]);
                                if matches!(
                                    s.stage,
                                    STAGE_HERMES | STAGE_DEMETER | STAGE_HEPHAESTUS | STAGE_HESTIA | STAGE_POSEIDON
                                ) {
                                    v += 200;
                                    if hb == hw && hb + 1 < DOME {
                                        v += 500;
                                    } else if hb == 2 && hw == 2 {
                                        v += 900;
                                    }
                                }
                            }
                        }
                        if a == killers[0] || a == killers[1] {
                            v += 1 << 22;
                        }
                        v
                    }
                    ActionKind::DomeOnly { w, d } => {
                        let mut v = self.history[p][a as usize] / 8;
                        let wsq = s.workers[p][w];
                        if wsq != UNPLACED {
                            let bsq = NEIGHBORS[wsq as usize][d];
                            if bsq != OFF_BOARD {
                                if opp_threat && blocks_threat(s, 1 - p, wsq as usize, bsq as usize) {
                                    v += 1 << 26;
                                } else if s.height[bsq as usize] == 2
                                    && s.workers[1 - p]
                                        .iter()
                                        .any(|&o| o != UNPLACED && NEIGHBORS[o as usize].contains(&bsq))
                                {
                                    v += 1_200;
                                }
                            }
                        }
                        if a == killers[0] || a == killers[1] {
                            v += 1 << 22;
                        }
                        v
                    }
                    ActionKind::Force { w, d } => {
                        let mut v = self.history[p][a as usize] / 8;
                        let wsq = s.workers[p][w];
                        if wsq != UNPLACED {
                            let opp_sq = NEIGHBORS[wsq as usize][d];
                            if opp_sq != OFF_BOARD {
                                v += 800 + 1_200 * s.height[opp_sq as usize] as i32;
                            }
                        }
                        if a == killers[0] || a == killers[1] {
                            v += 1 << 22;
                        }
                        v
                    }
                    ActionKind::Square(sq) => {
                        let mut v = self.history[p][a as usize] / 8;
                        if s.workers[1 - p].contains(&(sq as u8)) {
                            v += if s.stage == STAGE_SCYLLA { 800 + 1_500 * s.height[sq] as i32 } else { 8_000 };
                        } else if s.gods[p] == GOD_ARES {
                            v += if s.height[sq] >= 2 { 1_500 } else { 250 };
                        } else if s.height[sq] == 2
                            && s.workers[1 - p]
                                .iter()
                                .any(|&o| o != UNPLACED && NEIGHBORS[o as usize].contains(&(sq as u8)))
                        {
                            v += 1_200;
                        }
                        if a == killers[0] || a == killers[1] {
                            v += 1 << 22;
                        }
                        v
                    }
                }
            };
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn child(
        &mut self,
        parent: &CompactSantoriniState,
        c: &CompactSantoriniState,
        ch: u64,
        depth: i32,
        cply: usize,
        alpha: i32,
        beta: i32,
        is_pv: bool,
    ) -> i32 {
        if c.game_over {
            self.nodes += 1;
            return if c.winner == parent.current_player as i8 {
                MATE - cply as i32
            } else {
                -(MATE - cply as i32)
            };
        }
        if c.current_player == parent.current_player {
            self.negamax(c, ch, depth, cply, alpha, beta, is_pv, NO_MOVE)
        } else {
            -self.negamax(c, ch, depth, cply, -beta, -alpha, is_pv, NO_MOVE)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn negamax(
        &mut self,
        s: &CompactSantoriniState,
        hash: u64,
        mut depth: i32,
        ply: usize,
        mut alpha: i32,
        mut beta: i32,
        is_pv: bool,
        excluded: i32,
    ) -> i32 {
        self.nodes += 1;
        if self.check_abort() {
            return 0;
        }
        if s.game_over {
            return if s.winner == s.current_player as i8 { MATE - ply as i32 } else { -(MATE - ply as i32) };
        }
        if s.has_winning_move() {
            return MATE - (ply as i32 + 1);
        }
        alpha = alpha.max(-(MATE - ply as i32));
        beta = beta.min(MATE - ply as i32 - 1);
        if alpha >= beta {
            return alpha;
        }
        if depth <= 0 || ply >= MAX_PLY - 8 {
            return self.qsearch(s, hash, ply, alpha, beta, 0, 0, 0, &mut None);
        }

        let p = s.current_player as usize;
        let min_dome_sq: Option<u8> = match (s.stage, self.prev_move[ply]) {
            (STAGE_HERACLES, pm) if pm >= 0 => match decode_action(pm) {
                ActionKind::DomeOnly { w, d } if s.workers[p][w] != UNPLACED => {
                    let sq = NEIGHBORS[s.workers[p][w] as usize][d];
                    (sq != OFF_BOARD).then_some(sq + 1)
                }
                _ => None,
            },
            (STAGE_END, pm) if s.gods[p] == GOD_POLYPHEMUS && pm >= 0 => match decode_action(pm) {
                ActionKind::Square(sq) => Some(sq as u8 + 1),
                _ => None,
            },
            _ => None,
        };
        let tt = if excluded == NO_MOVE && min_dome_sq.is_none() { self.tt.probe(hash, ply) } else { None };
        let tt_move = tt.map_or(NO_MOVE, |e| e.action);
        if let Some(e) = tt {
            if !is_pv && e.depth as i32 >= depth {
                let cut = match e.flag {
                    TT_EXACT => true,
                    TT_LOWER => e.score >= beta,
                    TT_UPPER => e.score <= alpha,
                    _ => false,
                };
                if cut {
                    return e.score;
                }
            }
        }
        let opp_threat = s.has_winning_move_for(1 - p);
        let can_prune = !is_pv && s.sub_phase == PHASE_TURN && !opp_threat && excluded == NO_MOVE && beta.abs() < MATE_BOUND;
        let static_eval = if can_prune { self.eval(s, hash) } else { 0 };
        if can_prune && depth <= self.params.rfp_max_depth && static_eval - self.params.rfp_margin * depth >= beta {
            return static_eval;
        }
        if can_prune && depth >= 3 && static_eval >= beta && s.sub_phase == PHASE_TURN {
            let mut null = *s;
            let nh = if null.is_mortal_game() {
                null.current_player ^= 1;
                CompactSantoriniState::null_move_hash(hash)
            } else {
                null.status_flags &= !(1 << p);
                null.current_player ^= 1;
                null.begin_turn(1 - p);
                null.state_hash()
            };
            if null.is_mortal_game() || null.has_valid_action() {
                self.prev_move[ply + 1] = NO_MOVE;
                let r = self.params.nmp_base + depth / 3;
                let v = -self.negamax(&null, nh, depth - 1 - r, ply + 1, -beta, -beta + 1, false, NO_MOVE);
                if self.aborted {
                    return 0;
                }
                if v >= beta {
                    return if v >= MATE_BOUND { beta } else { v };
                }
            }
        }
        if depth >= 4 && tt_move == NO_MOVE && excluded == NO_MOVE {
            depth -= 1;
        }
        let futility = can_prune && depth <= 3 && static_eval + 600 * depth <= alpha;

        let mut moves = [0i32; MAX_ACTIONS];
        let mut scores = [0i32; MAX_ACTIONS];
        let n = s.write_valid_actions_into(&mut moves);
        self.score_moves(s, &moves[..n], &mut scores[..n], tt_move, ply, opp_threat);

        let orig_alpha = alpha;
        let mut best = -INF;
        let mut best_move = NO_MOVE;
        let mut searched = 0usize;
        let mut quiets_tried = [0i32; MAX_ACTIONS];
        let mut n_quiets = 0usize;
        let mut walk_pruned = false;
        self.walk_key[ply] = (s.workers[p], (s.turn_flags >> p) & 1);
        for i in 0..n {
            pick_next(&mut moves[..n], &mut scores[..n], i);
            let m = moves[i];
            if m == excluded {
                continue;
            }
            if s.stage == STAGE_HERACLES {
                if let ActionKind::DomeOnly { w, d } = decode_action(m) {
                    let bsq = NEIGHBORS[s.workers[p][w] as usize][d];
                    if w == 1 && s.workers[p][0] != UNPLACED && NEIGHBORS[s.workers[p][0] as usize].contains(&bsq) {
                        continue;
                    }
                    if min_dome_sq.is_some_and(|min_sq| bsq < min_sq) {
                        continue;
                    }
                }
            } else if let (Some(min_sq), ActionKind::Square(sq)) = (min_dome_sq, decode_action(m)) {
                if (sq as u8) < min_sq {
                    continue;
                }
            } else if s.stage == STAGE_HERMES && s.steps() == 1 && self.walk_moved_to[ply] != UNPLACED {
                if let ActionKind::BuildOnly { w, .. } = decode_action(m) {
                    if s.workers[p][w] == self.walk_moved_to[ply] {
                        walk_pruned = true;
                        continue;
                    }
                }
            }
            let quiet = scores[i] < 1 << 21 && !is_climb(s, m);
            if quiet && searched > 0 && best > -MATE_BOUND {
                if futility {
                    continue;
                }
                if !is_pv && !opp_threat && depth <= 5 {
                    let lmp = if s.stage == STAGE_HERACLES && min_dome_sq.is_some() {
                        2 + depth as usize
                    } else {
                        3 + (depth * depth) as usize
                    };
                    if searched >= lmp {
                        continue;
                    }
                    if depth <= 3 && self.history[p][m as usize] < -1024 * depth {
                        continue;
                    }
                }
            }
            let mut c = *s;
            c.apply_action(m);
            if matches!(s.stage, STAGE_HERMES | STAGE_CHAIN) && matches!(c.stage, STAGE_HERMES | STAGE_CHAIN) {
                let start = self.walk_start[ply];
                let first = if s.stage == STAGE_HERMES { start } else { start + 2 };
                let ck = (c.workers[p], (c.turn_flags >> p) & 1);
                if first <= ply && self.walk_key[first..=ply].contains(&ck) {
                    walk_pruned = true;
                    continue;
                }
            }
            let ch = s.child_hash(hash, m, &c);

            let same_side = c.current_player == s.current_player;
            let mut ext = 0;
            if !same_side && m == tt_move && depth >= 6 && excluded == NO_MOVE {
                if let Some(e) = tt {
                    if e.depth as i32 >= depth - 3 && e.flag != TT_UPPER && e.score.abs() < MATE_BOUND {
                        let sbeta = e.score - 40 * depth;
                        let v = self.negamax(s, hash, (depth - 1) / 2, ply, sbeta - 1, sbeta, false, m);
                        if self.aborted {
                            return 0;
                        }
                        if v < sbeta {
                            ext = 1;
                        }
                    }
                }
            }
            let new_depth = if same_side { depth } else { depth - 1 + ext };
            self.prev_move[ply + 1] = m;
            self.walk_key[ply] = (s.workers[p], (s.turn_flags >> p) & 1);
            self.walk_start[ply + 1] = if same_side { self.walk_start[ply] } else { ply + 1 };
            self.walk_moved_to[ply + 1] = match (s.stage, s.gods[p], s.gods[1 - p], decode_action(m)) {
                (STAGE_NONE, GOD_HERMES, og, ActionKind::MoveOnly { w, m: dir }) if og != GOD_HARPIES => {
                    NEIGHBORS[s.workers[p][w] as usize][dir]
                }
                _ => UNPLACED,
            };

            let v = if searched == 0 {
                self.child(s, &c, ch, new_depth, ply + 1, alpha, beta, is_pv)
            } else {
                let r = if same_side && !opp_threat {
                    let step_pen = if !is_climb(s, m) {
                        (ply - self.walk_start[ply])
                            .max(s.steps() as usize)
                            .max((s.sub_phase == PHASE_SUB_TURN) as usize) as i32
                    } else {
                        0
                    };
                    (lmr(&self.params, depth, searched.max(2), is_pv).max(1) + step_pen).min(new_depth).max(0)
                } else if quiet && !opp_threat {
                    lmr(&self.params, depth, searched, is_pv).min(new_depth - 1).max(0)
                } else {
                    0
                };
                let mut v = self.child(s, &c, ch, new_depth - r, ply + 1, alpha, alpha + 1, false);
                if v > alpha && r > 0 && !self.aborted {
                    v = self.child(s, &c, ch, new_depth, ply + 1, alpha, alpha + 1, false);
                }
                if v > alpha && v < beta && is_pv && !self.aborted {
                    v = self.child(s, &c, ch, new_depth, ply + 1, alpha, beta, true);
                }
                v
            };
            if self.aborted {
                return 0;
            }
            searched += 1;
            if quiet {
                quiets_tried[n_quiets] = m;
                n_quiets += 1;
            }
            if v > best {
                best = v;
                best_move = m;
                if v > alpha {
                    alpha = v;
                    if v >= beta {
                        if quiet {
                            if ply < MAX_PLY && self.killers[ply][0] != m {
                                self.killers[ply][1] = self.killers[ply][0];
                                self.killers[ply][0] = m;
                            }
                            let bonus = depth * depth * 16;
                            let h = &mut self.history[p][m as usize];
                            *h = (*h + bonus).min(1 << 20);
                            for &q in &quiets_tried[..n_quiets - 1] {
                                let h = &mut self.history[p][q as usize];
                                *h = (*h - bonus).max(-(1 << 20));
                            }
                            if self.prev_move[ply] >= 0 {
                                self.counter[p][self.prev_move[ply] as usize] = m;
                            }
                        }
                        break;
                    }
                }
            }
        }
        if best_move == NO_MOVE {
            return alpha;
        }
        if excluded == NO_MOVE && min_dome_sq.is_none() && !walk_pruned {
            let flag = if best >= beta {
                TT_LOWER
            } else if best > orig_alpha {
                TT_EXACT
            } else {
                TT_UPPER
            };
            self.tt.store(hash, best, best_move, depth as u8, flag, ply);
        }
        best
    }

    #[allow(clippy::too_many_arguments)]
    fn qsearch(
        &mut self,
        s: &CompactSantoriniState,
        hash: u64,
        ply: usize,
        mut alpha: i32,
        beta: i32,
        qdepth: u32,
        sub_ply: u8,
        min_sub_action: i32,
        walk_seen: &mut Option<Box<[[u8; 52]; 26]>>,
    ) -> i32 {
        self.nodes += 1;
        if self.check_abort() {
            return 0;
        }
        if s.game_over {
            return if s.winner == s.current_player as i8 { MATE - ply as i32 } else { -(MATE - ply as i32) };
        }
        if s.has_winning_move() {
            return MATE - (ply as i32 + 1);
        }
        let p = s.current_player as usize;
        if qdepth >= 4 || ply >= MAX_PLY - 2 || !s.has_winning_move_for(1 - p) {
            return self.eval(s, hash);
        }
        let sub_cap = if s.stage == STAGE_HERACLES { 2 } else { 6 };
        if sub_ply >= sub_cap {
            return -(MATE - ply as i32 - 2);
        }
        if matches!(s.stage, STAGE_CHAIN | STAGE_HERMES) {
            let w0 = (s.workers[p][0] as usize).min(25);
            let w1 = (s.workers[p][1] as usize).min(25);
            let col = w1 * 2 + (((s.turn_flags >> p) & 1) as usize);
            let step_tag = s.steps() + 1;
            let table = walk_seen.get_or_insert_with(|| Box::new([[0u8; 52]; 26]));
            let prev = table[w0][col];
            if prev != 0 && prev <= step_tag {
                return -(MATE - ply as i32 - 2);
            }
            table[w0][col] = step_tag;
        }
        let comm_domes = s.stage == STAGE_HERACLES || (s.stage == STAGE_END && s.gods[p] == GOD_POLYPHEMUS);
        let mut moves = [0i32; MAX_ACTIONS];
        let n = s.write_valid_actions_into(&mut moves);
        let mut best = -(MATE - ply as i32 - 2);
        let mut child_walk: Option<Box<[[u8; 52]; 26]>> = None;
        for &m in &moves[..n] {
            if comm_domes && m != PASS_ACTION && m < min_sub_action {
                continue;
            }
            let mut c = *s;
            c.apply_action(m);
            if c.game_over {
                if c.winner == s.current_player as i8 {
                    return MATE - (ply as i32 + 1);
                }
                continue;
            }
            let same_side = c.current_player == s.current_player;
            if !same_side && c.has_winning_move() {
                continue;
            }
            let next_min = if same_side && comm_domes && m != PASS_ACTION { m + 1 } else { 0 };
            let ch = s.child_hash(hash, m, &c);
            let v = if same_side {
                let ws = if matches!(s.stage, STAGE_CHAIN | STAGE_HERMES) { &mut *walk_seen } else { &mut child_walk };
                self.qsearch(&c, ch, ply + 1, alpha, beta, qdepth, sub_ply + 1, next_min, ws)
            } else {
                -self.qsearch(&c, ch, ply + 1, -beta, -alpha, qdepth + 1, 0, 0, &mut None)
            };
            if self.aborted {
                return 0;
            }
            if v > best {
                best = v;
                if v > alpha {
                    alpha = v;
                    if v >= beta {
                        break;
                    }
                }
            }
        }
        best
    }
}

#[inline]
fn blocks_threat(s: &CompactSantoriniState, threatener: usize, to: usize, bsq: usize) -> bool {
    let pan = s.gods[threatener] == GOD_PAN;
    let near = |sq: usize| {
        let win_h = s.height[sq] == 3 || (pan && s.height[sq] == 0);
        win_h
            && s.workers[threatener]
                .iter()
                .any(|&w| w != UNPLACED && s.height[w as usize] == 2 && NEIGHBORS[w as usize].contains(&(sq as u8)))
    };
    near(to) || near(bsq)
}

#[inline(always)]
fn step_dest(s: &CompactSantoriniState, p: usize, from: usize, m: usize) -> u8 {
    if s.is_mortal_game() {
        return NEIGHBORS[from][m];
    }
    let god = s.gods[p];
    let n1 = if god == GOD_URANIA { WRAP_NEIGHBORS[from][m] } else { NEIGHBORS[from][m] };
    if god == GOD_IRIS && n1 != OFF_BOARD && (s.workers[0].contains(&n1) || s.workers[1].contains(&n1)) {
        NEIGHBORS[n1 as usize][m]
    } else {
        n1
    }
}

#[inline(always)]
fn build_dest(s: &CompactSantoriniState, p: usize, w: usize, to: usize, b: usize, dome: bool) -> u8 {
    if s.is_mortal_game() {
        return NEIGHBORS[to][b];
    }
    let god = s.gods[p];
    if dome && god == GOD_SELENE {
        let fsq = if s.tags[p][w] & TAG_FEMALE != 0 {
            to as u8
        } else {
            s.workers[p][1 - w]
        };
        if fsq == UNPLACED {
            OFF_BOARD
        } else {
            NEIGHBORS[fsq as usize][b]
        }
    } else if god == GOD_URANIA {
        WRAP_NEIGHBORS[to][b]
    } else {
        NEIGHBORS[to][b]
    }
}

#[inline(always)]
fn is_climb(s: &CompactSantoriniState, a: i32) -> bool {
    let p = s.current_player as usize;
    match decode_action(a) {
        ActionKind::Turn { w, m, .. } => {
            let from = s.workers[p][w] as usize;
            let to = step_dest(s, p, from, m);
            if to == OFF_BOARD {
                return false;
            }
            if s.height[to as usize] > s.height[from] {
                return true;
            }
            if !s.is_mortal_game() && s.gods[p] == GOD_BIA {
                let next = NEIGHBORS[to as usize][m];
                return next != OFF_BOARD && s.workers[1 - p].contains(&next);
            }
            false
        }
        ActionKind::MoveOnly { w, m } => {
            let from = s.workers[p][w] as usize;
            let to = step_dest(s, p, from, m);
            to != OFF_BOARD && s.height[to as usize] > s.height[from]
        }
        ActionKind::Zeus { w, m } => {
            let from = s.workers[p][w] as usize;
            let to = NEIGHBORS[from][m];
            to != OFF_BOARD && s.height[to as usize] + 1 > s.height[from]
        }
        ActionKind::Force { w, d } => {
            let wsq = s.workers[p][w];
            wsq != UNPLACED && {
                let opp_sq = NEIGHBORS[wsq as usize][d];
                opp_sq != OFF_BOARD && s.height[opp_sq as usize] >= 1
            }
        }
        ActionKind::Square(sq) => s.stage != STAGE_SCYLLA && s.workers[1 - p].contains(&(sq as u8)),
        _ => false,
    }
}

#[inline(always)]
fn pick_next(moves: &mut [i32], scores: &mut [i32], i: usize) {
    let mut best = i;
    for j in i + 1..moves.len() {
        if scores[j] > scores[best] {
            best = j;
        }
    }
    moves.swap(i, best);
    scores.swap(i, best);
}
