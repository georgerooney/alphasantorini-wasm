//! Zero-allocation compact Santorini engine (`CompactSantoriniState`).
//!
//! Why `CompactSantoriniState`: MCTS self-play and the Lazy-SMP alpha-beta engine copy
//! states millions of times per second, so the whole game lives in a `<= 64`-byte `Copy`
//! struct (`STATE_BYTES = 51` serialized) with no heap, legal moves are written into a
//! caller-owned stack buffer, and the Zobrist hash is order-invariant over a player's two
//! workers. Base (Mortal) turns use bitboard fast paths here; god rules delegate to `gods.rs`.
//!
//! # Action encoding (`ACTION_SPACE = 153`)
//! * Setup plies (`PHASE_PLACE_*`): `action = square` in `0..25`.
//! * Turn (`PHASE_TURN`): `25 + worker * 64 + move_dir * 8 + build_dir`, where `worker`
//!   indexes the *sorted* pair (`workers[p][0] < workers[p][1]`).
//! * A winning move (stepping up 2 -> 3) does not build. It is emitted exactly once,
//!   with `build_dir = opposite(move_dir)`. Why that direction: it is always on-board
//!   and always a legal build target (the vacated origin), and because every D4
//!   symmetry is linear it commutes with `opposite`, so the encoding stays equivariant.

pub const BOARD_SIZE: usize = 5;
pub const NUM_SQUARES: usize = BOARD_SIZE * BOARD_SIZE;
pub const DOME: u8 = 4;
pub const UNPLACED: u8 = 255;
pub const OFF_BOARD: u8 = 255;

pub const NUM_PLACEMENT_ACTIONS: usize = NUM_SQUARES;
pub const TURN_ACTION_OFFSET: i32 = NUM_PLACEMENT_ACTIONS as i32;
/// Full action space: placement (0..25), Base turns (25..153) and the god extensions
/// (153..387, see `gods.rs`). IDs are append-only: Base IDs are unchanged from Phase 1-3 and
/// the intro-god IDs from Phase 4.
pub const ACTION_SPACE: usize = crate::gods::ACTION_END as usize;
/// Number of Base-game actions (placement + Base turns).
pub const BASE_ACTION_SPACE: usize = NUM_PLACEMENT_ACTIONS + 2 * 8 * 8;
// Observation v3 layout (side-to-move relative, D4-equivariant):
//   0..=4  height 0..=4 one-hot
//   5..=6  my workers, opponent workers
//   7      all-ones during setup
//   8      worker bound to the current sub-turn (`stage_binds_worker`)
//   9      sub-turn `prev_square`
//   10..=11 Athena restriction on me / on opponent (all-ones)
//   12..=13 female worker marker (Selene / Hippolyta): mine, opponent
//   14..=17 current-turn tags on my workers: frozen (Hypnus), Aphrodite, moved, built
//   18      low domes (`low_domes` bitmask: domes built below level 3, for Chronus)
//   19..=20 once-per-game power spent: mine, opponent (all-ones)
//   21..=23 current-turn flags on me: moved up, moved down, Persephone obligation (all-ones)
//   24..40  active sub-turn stage one-hot (`1..NUM_STAGES`, all-ones on active plane)
//   40      chain / multi-step counter (`steps() / CHAIN_MAX_STEPS` across all 25 squares)
// Followed by 6 scalars: `[my_w0, my_w1, opp_w0, opp_w1, my_god_idx, opp_god_idx]` (`25` =
// unplaced; `*_god_idx` is the dense `0..NUM_SUPPORTED_GODS` index from `gods::GOD_INDEX`).
// Why planes 12..41 are spatial/broadcast planes instead of trailing scalars: the ResNet stem
// (`Conv2d(..., bias=False)`) sees sub-turn stage, step count, tags, and low domes from layer 1
// (replacing the ignored `OBS_SPATIAL + 6` step scalar), while zero-padding `stem.0.weight`
// preserves legacy Mortal (8-plane) and intro-god (12-plane) evaluations bit-for-bit.
pub const OBS_PLANE_MY_WORKERS: usize = 5;
pub const OBS_PLANE_OPP_WORKERS: usize = 6;
pub const OBS_PLANE_SETUP: usize = 7;
pub const OBS_PLANE_ACTIVE: usize = 8;
pub const OBS_PLANE_PREV: usize = 9;
pub const OBS_PLANE_ME_RESTRICTED: usize = 10;
pub const OBS_PLANE_OPP_RESTRICTED: usize = 11;
pub const OBS_PLANE_MY_FEMALE: usize = 12;
pub const OBS_PLANE_OPP_FEMALE: usize = 13;
pub const OBS_PLANE_MY_FROZEN: usize = 14;
pub const OBS_PLANE_MY_APHRODITE: usize = 15;
pub const OBS_PLANE_MY_MOVED: usize = 16;
pub const OBS_PLANE_MY_BUILT: usize = 17;
pub const OBS_PLANE_LOW_DOMES: usize = 18;
pub const OBS_PLANE_ME_POWER_USED: usize = 19;
pub const OBS_PLANE_OPP_POWER_USED: usize = 20;
pub const OBS_PLANE_MOVED_UP: usize = 21;
pub const OBS_PLANE_MOVED_DOWN: usize = 22;
pub const OBS_PLANE_PERSEPHONE: usize = 23;
pub const OBS_PLANE_STAGE_BASE: usize = 24;
pub const OBS_PLANE_STEPS: usize = OBS_PLANE_STAGE_BASE + (crate::gods::NUM_STAGES - 1);
pub const OBS_PLANES: usize = OBS_PLANE_STEPS + 1;
pub const OBS_SPATIAL: usize = OBS_PLANES * NUM_SQUARES;
pub const OBS_LEN: usize = OBS_SPATIAL + 6;
/// Size of `CompactSantoriniState::to_bytes`.
pub const STATE_BYTES: usize = 51;

/// Upper bound on legal actions in any state, gods included (Atlas: 128 block + 128 dome
/// turns; bounded by the action space).
pub const MAX_ACTIONS: usize = 320;
pub const NUM_SYMMETRIES: usize = 8;

pub const PHASE_PLACE_0: u8 = 0;
pub const PHASE_PLACE_1: u8 = 1;
pub const PHASE_TURN: u8 = 2;
/// Reserved for multi-step gods (Artemis / Demeter / Prometheus), Phase 4.
pub const PHASE_SUB_TURN: u8 = 3;

pub const DIR_N: usize = 0;
pub const DIR_NE: usize = 1;
pub const DIR_E: usize = 2;
pub const DIR_SE: usize = 3;
pub const DIR_S: usize = 4;
pub const DIR_SW: usize = 5;
pub const DIR_W: usize = 6;
pub const DIR_NW: usize = 7;
/// `(d_row, d_col)` per direction; row 0 is the top (north) edge.
pub const DIR_DELTAS: [(i32, i32); 8] = [(-1, 0), (-1, 1), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1)];

// ---------------------------------------------------------------------------
// Compile-time tables
// ---------------------------------------------------------------------------

const fn dir_index(dr: i32, dc: i32) -> u8 {
    let mut d = 0;
    while d < 8 {
        if DIR_DELTAS[d].0 == dr && DIR_DELTAS[d].1 == dc {
            return d as u8;
        }
        d += 1;
    }
    panic!("not a unit direction")
}

/// `NEIGHBORS[sq][dir]` = adjacent square, or `OFF_BOARD`.
pub const NEIGHBORS: [[u8; 8]; NUM_SQUARES] = {
    let mut t = [[OFF_BOARD; 8]; NUM_SQUARES];
    let mut sq = 0;
    while sq < NUM_SQUARES {
        let (r, c) = ((sq / BOARD_SIZE) as i32, (sq % BOARD_SIZE) as i32);
        let mut d = 0;
        while d < 8 {
            let (nr, nc) = (r + DIR_DELTAS[d].0, c + DIR_DELTAS[d].1);
            if nr >= 0 && nr < BOARD_SIZE as i32 && nc >= 0 && nc < BOARD_SIZE as i32 {
                t[sq][d] = (nr as usize * BOARD_SIZE + nc as usize) as u8;
            }
            d += 1;
        }
        sq += 1;
    }
    t
};

/// `NEIGHBOR_BIT[sq][dir]` = `1 << NEIGHBORS[sq][dir]`, or 0 off-board.
pub const NEIGHBOR_BIT: [[u32; 8]; NUM_SQUARES] = {
    let mut t = [[0u32; 8]; NUM_SQUARES];
    let mut sq = 0;
    while sq < NUM_SQUARES {
        let mut d = 0;
        while d < 8 {
            if NEIGHBORS[sq][d] != OFF_BOARD {
                t[sq][d] = 1 << NEIGHBORS[sq][d];
            }
            d += 1;
        }
        sq += 1;
    }
    t
};

/// `NEIGHBOR_MASK[sq]` = bitmask of all squares adjacent to `sq`.
pub const NEIGHBOR_MASK: [u32; NUM_SQUARES] = {
    let mut t = [0u32; NUM_SQUARES];
    let mut sq = 0;
    while sq < NUM_SQUARES {
        let mut d = 0;
        while d < 8 {
            t[sq] |= NEIGHBOR_BIT[sq][d];
            d += 1;
        }
        sq += 1;
    }
    t
};

/// D4 element `sym = reflect * 4 + rot`: optional left-right mirror, then `rot`
/// clockwise quarter turns. Applied to centred coordinates `(y, x)` in `-2..=2`.
const fn d4_apply(sym: usize, y: i32, x: i32) -> (i32, i32) {
    let (mut y, mut x) = if sym >= 4 { (y, -x) } else { (y, x) };
    let mut k = 0;
    while k < sym % 4 {
        // Clockwise quarter turn: (y, x) -> (x, -y). E.g. N (-1,0) -> E (0,1).
        let t = y;
        y = x;
        x = -t;
        k += 1;
    }
    (y, x)
}

/// `PERMUTE_SQ[sym][sq]` = image of `sq` under symmetry `sym`.
pub const PERMUTE_SQ: [[u8; NUM_SQUARES]; NUM_SYMMETRIES] = {
    let mut t = [[0u8; NUM_SQUARES]; NUM_SYMMETRIES];
    let mut s = 0;
    while s < NUM_SYMMETRIES {
        let mut sq = 0;
        while sq < NUM_SQUARES {
            let (y, x) = d4_apply(s, (sq / BOARD_SIZE) as i32 - 2, (sq % BOARD_SIZE) as i32 - 2);
            t[s][sq] = ((y + 2) as usize * BOARD_SIZE + (x + 2) as usize) as u8;
            sq += 1;
        }
        s += 1;
    }
    t
};

/// `PERMUTE_DIR[sym][dir]` = image of compass direction `dir` under symmetry `sym`.
pub const PERMUTE_DIR: [[u8; 8]; NUM_SYMMETRIES] = {
    let mut t = [[0u8; 8]; NUM_SYMMETRIES];
    let mut s = 0;
    while s < NUM_SYMMETRIES {
        let mut d = 0;
        while d < 8 {
            let (dy, dx) = d4_apply(s, DIR_DELTAS[d].0, DIR_DELTAS[d].1);
            t[s][d] = dir_index(dy, dx);
            d += 1;
        }
        s += 1;
    }
    t
};

/// `INVERSE_SYM[sym]` = the symmetry undoing `sym`.
pub const INVERSE_SYM: [u8; NUM_SYMMETRIES] = {
    let mut t = [0u8; NUM_SYMMETRIES];
    let mut a = 0;
    while a < NUM_SYMMETRIES {
        let mut b = 0;
        while b < NUM_SYMMETRIES {
            let mut identity = true;
            let mut sq = 0;
            while sq < NUM_SQUARES {
                if PERMUTE_SQ[b][PERMUTE_SQ[a][sq] as usize] as usize != sq {
                    identity = false;
                }
                sq += 1;
            }
            if identity {
                t[a] = b as u8;
            }
            b += 1;
        }
        a += 1;
    }
    t
};

// ---------------------------------------------------------------------------
// Zobrist keys (deterministic SplitMix64, so hashes are bit-exact across builds,
// processes and the Python boundary -- required for shared/persisted TTs)
// ---------------------------------------------------------------------------

const fn splitmix64(state: u64) -> (u64, u64) {
    let s = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (s, z ^ (z >> 31))
}

// Unified State v2 Zobrist table (nothing persists hashes across runs, so all gods share Z_GOD).
const ZOBRIST_LEN: usize = Z_LOW_DOME + NUM_SQUARES;
const Z_HEIGHT: usize = 0; // [sq * 5 + h]; h == 0 is never XORed
const Z_WORKER: usize = Z_HEIGHT + NUM_SQUARES * 5; // [p * 25 + sq]
const Z_SIDE: usize = Z_WORKER + 2 * NUM_SQUARES;
const Z_PHASE: usize = Z_SIDE + 1;
const Z_GOD: usize = Z_PHASE + 4; // [p * NUM_GODS + god]; god 0 (Mortal) never XORed
const Z_FLAG: usize = Z_GOD + 2 * crate::gods::NUM_GODS; // [bit]
const Z_ACTIVE: usize = Z_FLAG + 8; // [worker], only in worker-bound stages
const Z_PREV: usize = Z_ACTIVE + 2; // [sq], only when set
const Z_POWER: usize = Z_PREV + NUM_SQUARES; // [p]
const Z_TURN_FLAG: usize = Z_POWER + 2; // [bit]
const Z_STAGE: usize = Z_TURN_FLAG + 8; // [stage - 1] for stage in 1..NUM_STAGES
const Z_TAG: usize = Z_STAGE + (crate::gods::NUM_STAGES - 1); // [(p * 25 + sq) * 8 + bit], keyed by the tagged worker's square
const Z_LOW_DOME: usize = Z_TAG + 2 * NUM_SQUARES * 8; // [sq]

const ZOBRIST: [u64; ZOBRIST_LEN] = {
    let mut t = [0u64; ZOBRIST_LEN];
    let mut state = 0x5A17_0815_A11C_E5EDu64;
    let mut i = 0;
    while i < ZOBRIST_LEN {
        let (s, v) = splitmix64(state);
        state = s;
        t[i] = v;
        i += 1;
    }
    t
};

// ---------------------------------------------------------------------------
// Action helpers
// ---------------------------------------------------------------------------

#[inline(always)]
pub const fn encode_turn_action(worker: usize, move_dir: usize, build_dir: usize) -> i32 {
    TURN_ACTION_OFFSET + (worker * 64 + move_dir * 8 + build_dir) as i32
}

/// Inverse of [`encode_turn_action`]; `action` must be `>= TURN_ACTION_OFFSET`.
#[inline(always)]
pub const fn decode_turn_action(action: i32) -> (usize, usize, usize) {
    let a = (action - TURN_ACTION_OFFSET) as usize;
    (a / 64, (a / 8) % 8, a % 8)
}

#[inline(always)]
pub const fn opposite_dir(dir: usize) -> usize {
    (dir + 4) & 7
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct CompactSantoriniState {
    /// 0 = ground, 1..=3 = levels, 4 = dome.
    pub height: [u8; NUM_SQUARES],
    /// `[player][i]` square or `UNPLACED`; always sorted ascending (UNPLACED last).
    pub workers: [[u8; 2]; 2],
    /// God ID per player (0 = Mortal). Reserved: Phase 1 only supports Mortal.
    pub gods: [u8; 2],
    /// Turn-persistent god effects (bit 0: Athena moved up). Reserved.
    pub status_flags: u8,
    pub sub_phase: u8,
    /// Locked worker during a multi-step god sub-turn. Reserved.
    pub active_worker: u8,
    /// Artemis/Demeter restriction square, or `UNPLACED`. Reserved.
    pub prev_square: u8,
    pub current_player: u8,
    pub game_over: bool,
    /// -1 while in progress, else the winning player.
    pub winner: i8,
    /// Pantheon bookkeeping (all zero in Mortal / intro-god games, see `gods.rs`):
    /// bit p = player p spent a once-per-game power.
    pub power_used: u8,
    /// Moved up / down during the latest turn, Persephone obligation (`gods::TF_*`).
    pub turn_flags: u8,
    /// Pantheon sub-turn stage (`gods::STAGE_*`); 0 outside pantheon sub-turns.
    pub stage: u8,
    /// Per-worker tags parallel to `workers` (female, Aphrodite/Hypnus marks, ...); they are
    /// permuted with the workers by `sort_workers`.
    pub tags: [[u8; 2]; 2],
    /// Plies elapsed (setup plies included). Excluded from the hash.
    pub turn_count: u16,
    /// Domes standing on a level below 3 (Atlas-style), so Chronus can count complete towers.
    pub low_domes: u32,
}

const _: () = assert!(std::mem::size_of::<CompactSantoriniState>() <= 64);

impl Default for CompactSantoriniState {
    fn default() -> Self {
        Self::new()
    }
}

impl CompactSantoriniState {
    /// Empty board, player 0 to place the first worker.
    pub const fn new() -> Self {
        Self {
            height: [0; NUM_SQUARES],
            workers: [[UNPLACED; 2]; 2],
            gods: [0; 2],
            status_flags: 0,
            sub_phase: PHASE_PLACE_0,
            active_worker: 0,
            prev_square: UNPLACED,
            current_player: 0,
            game_over: false,
            winner: -1,
            power_used: 0,
            turn_flags: 0,
            stage: 0,
            tags: [[0; 2]; 2],
            turn_count: 0,
            low_domes: 0,
        }
    }

    /// Builds a mid-game (`PHASE_TURN`) position, e.g. from a custom board editor or a
    /// test fixture. Workers are sorted and terminal status is derived, so the result
    /// satisfies every engine invariant. `turn_count` is inferred from blocks built.
    pub fn from_position(height: [u8; NUM_SQUARES], workers: [[u8; 2]; 2], current_player: u8) -> Self {
        let mut s = Self::new();
        s.height = height;
        s.workers = workers;
        s.sort_workers();
        s.sub_phase = PHASE_TURN;
        s.current_player = current_player;
        s.turn_count = 4 + height.iter().map(|&h| h as u16).sum::<u16>();
        for p in 0..2 {
            if s.workers[p].iter().any(|&w| height[w as usize] == 3) {
                s.game_over = true;
                s.winner = p as i8;
            }
        }
        if !s.game_over && !s.has_valid_action() {
            s.game_over = true;
            s.winner = 1 - current_player as i8;
        }
        s
    }

    #[inline(always)]
    pub(crate) fn sort_workers(&mut self) {
        for (w, t) in self.workers.iter_mut().zip(self.tags.iter_mut()) {
            if w[0] > w[1] {
                w.swap(0, 1);
                t.swap(0, 1);
            }
        }
    }

    /// `masks[h]` = bitmask of squares at height `h` (0..=4).
    #[inline(always)]
    pub fn level_masks(&self) -> [u32; 5] {
        let mut m = [0u32; 5];
        for (sq, &h) in self.height.iter().enumerate() {
            m[h as usize] |= 1 << sq;
        }
        m
    }

    /// Bitmask of squares holding any worker.
    #[inline(always)]
    pub(crate) fn occupied_mask(&self) -> u32 {
        let mut m = 0u32;
        for &w in self.workers.iter().flatten() {
            if w != UNPLACED {
                m |= 1 << w;
            }
        }
        m
    }

    /// Visits every legal action in canonical order; `f` returns `false` to stop early.
    /// Why one generator with a visitor: `write_valid_actions_into` and the
    /// early-exit `has_valid_action` (run after every turn for the no-move loss) must
    /// never disagree, and monomorphisation makes the closure free.
    #[inline(always)]
    fn for_each_action(&self, mut f: impl FnMut(i32) -> bool) {
        if self.game_over {
            return;
        }
        let occ = self.occupied_mask();
        if self.sub_phase >= PHASE_TURN && !self.is_mortal_game() {
            return self.for_each_action_gods(&mut f);
        }
        if self.sub_phase != PHASE_TURN {
            for sq in 0..NUM_SQUARES {
                if occ & (1 << sq) == 0 && self.placement_ok(sq) && !f(sq as i32) {
                    return;
                }
            }
            return;
        }
        // Bitboard formulation: one mask test per candidate instead of several loads and
        // compares (movegen measured 400-750 ns/node with the nested-branch version).
        let lv = self.level_masks();
        let not_dome = !lv[DOME as usize];
        for (w, &from) in self.workers[self.current_player as usize].iter().enumerate() {
            let hf = self.height[from as usize] as usize;
            // Reachable heights: 0..=hf+1 (never a dome).
            let mut reach = 0u32;
            for &m in &lv[..(hf + 2).min(DOME as usize)] {
                reach |= m;
            }
            let movable = NEIGHBOR_MASK[from as usize] & reach & !occ;
            if movable == 0 {
                continue;
            }
            let win_targets = if hf == 2 { lv[3] } else { 0 };
            for m in 0..8 {
                let to_bit = NEIGHBOR_BIT[from as usize][m];
                if movable & to_bit == 0 {
                    continue;
                }
                if win_targets & to_bit != 0 {
                    if !f(encode_turn_action(w, m, opposite_dir(m))) {
                        return;
                    }
                    continue;
                }
                let to = NEIGHBORS[from as usize][m] as usize;
                let buildable = NEIGHBOR_MASK[to] & not_dome & !((occ & !(1 << from)) | to_bit);
                for b in 0..8 {
                    if buildable & NEIGHBOR_BIT[to][b] != 0 && !f(encode_turn_action(w, m, b)) {
                        return;
                    }
                }
            }
        }
    }

    /// Writes all legal actions into `out` (no allocation) and returns the count.
    /// Returns 0 iff `game_over`.
    #[inline]
    pub fn write_valid_actions_into(&self, out: &mut [i32; MAX_ACTIONS]) -> usize {
        let mut n = 0;
        self.for_each_action(|a| {
            out[n] = a;
            n += 1;
            true
        });
        n
    }

    /// Cheap single-action legality check (for TT moves, which may come from another
    /// thread or a key collision). Agrees with `write_valid_actions_into` (tested).
    #[inline]
    pub fn is_legal_action(&self, a: i32) -> bool {
        if self.game_over || !(0..ACTION_SPACE as i32).contains(&a) {
            return false;
        }
        if self.sub_phase >= PHASE_TURN && !self.is_mortal_game() {
            let mut found = false;
            self.for_each_action_gods(&mut |x| {
                found = x == a;
                !found && x < a
            });
            return found;
        }
        let occ = self.occupied_mask();
        if self.sub_phase != PHASE_TURN {
            return a < TURN_ACTION_OFFSET && occ & (1 << a) == 0 && self.placement_ok(a as usize);
        }
        if a < TURN_ACTION_OFFSET || a >= BASE_ACTION_SPACE as i32 {
            return false;
        }
        let (w, m, b) = decode_turn_action(a);
        let from = self.workers[self.current_player as usize][w] as usize;
        let to = NEIGHBORS[from][m];
        if to == OFF_BOARD || occ & (1 << to) != 0 {
            return false;
        }
        let (hf, ht) = (self.height[from], self.height[to as usize]);
        if ht >= DOME || ht > hf + 1 {
            return false;
        }
        if hf == 2 && ht == 3 {
            return b == opposite_dir(m);
        }
        let t = NEIGHBORS[to as usize][b];
        t != OFF_BOARD && (occ & !(1 << from)) & (1 << t) == 0 && self.height[t as usize] < DOME
    }

    /// Whether the side to move has any legal action (run after every turn for the
    /// no-move loss rule).
    ///
    /// Why not reuse the generator: every legal *move* has at least one legal *build* —
    /// the vacated origin square, which is free and not a dome (a worker stood on it).
    /// So "has a turn" reduces to "some worker has a legal step", which exits on the
    /// first hit without building level masks. Equivalence with the full generator is
    /// asserted over 200k positions in `tests/compact_invariants.rs`.
    #[inline]
    pub fn has_valid_action(&self) -> bool {
        if self.game_over {
            return false;
        }
        if self.sub_phase < PHASE_TURN {
            return true; // at most 3 of 25 squares are occupied during setup
        }
        if !self.is_mortal_game() {
            let mut any = false;
            self.for_each_action_gods(&mut |_| {
                any = true;
                false
            });
            return any;
        }
        let occ = self.occupied_mask();
        self.workers[self.current_player as usize].iter().any(|&from| {
            let limit = self.height[from as usize] + 1;
            NEIGHBORS[from as usize]
                .iter()
                .any(|&n| n != OFF_BOARD && occ & (1 << n) == 0 && self.height[n as usize] <= limit && self.height[n as usize] < DOME)
        })
    }

    /// True if the side to move can step up onto level 3 right now (mate-in-1).
    #[inline]
    pub fn has_winning_move(&self) -> bool {
        self.has_winning_move_for(self.current_player as usize)
    }

    /// True if `player` could win by moving if it were their move (used to detect the
    /// opponent's mate threat without making a null move). Exact for the side to move in
    /// god games too (Artemis two-step climbs, Pan drops, Apollo/Minotaur entries, Athena),
    /// including mid-sub-turn (`PHASE_SUB_TURN`) so search/quiescence cannot hide from an
    /// opponent's mate-in-1 by entering an optional or multi-step sub-turn.
    #[inline]
    pub fn has_winning_move_for(&self, player: usize) -> bool {
        if self.game_over || self.sub_phase < PHASE_TURN {
            return false;
        }
        if !self.is_mortal_game() {
            return self.has_winning_move_gods(player);
        }
        let occ = self.occupied_mask();
        self.workers[player].iter().any(|&w| {
            self.height[w as usize] == 2
                && NEIGHBORS[w as usize]
                    .iter()
                    .any(|&n| n != OFF_BOARD && self.height[n as usize] == 3 && occ & (1 << n) == 0)
        })
    }

    /// Fixed `STATE_BYTES` little-endian serialisation (FFI / dataset storage).
    pub fn to_bytes(&self) -> [u8; STATE_BYTES] {
        let mut b = [0u8; STATE_BYTES];
        b[..25].copy_from_slice(&self.height);
        b[25..29].copy_from_slice(&[self.workers[0][0], self.workers[0][1], self.workers[1][0], self.workers[1][1]]);
        b[29..31].copy_from_slice(&self.gods);
        b[31] = self.status_flags;
        b[32] = self.sub_phase;
        b[33] = self.active_worker;
        b[34] = self.prev_square;
        b[35] = self.current_player;
        b[36] = self.game_over as u8;
        b[37] = self.winner as u8;
        b[38..40].copy_from_slice(&self.turn_count.to_le_bytes());
        b[40] = self.power_used;
        b[41] = self.turn_flags;
        b[42] = self.stage;
        b[43..47].copy_from_slice(&[self.tags[0][0], self.tags[0][1], self.tags[1][0], self.tags[1][1]]);
        b[47..51].copy_from_slice(&self.low_domes.to_le_bytes());
        b
    }

    pub fn from_bytes(b: &[u8; STATE_BYTES]) -> Self {
        let mut s = Self::new();
        s.height.copy_from_slice(&b[..25]);
        s.workers = [[b[25], b[26]], [b[27], b[28]]];
        s.gods = [b[29], b[30]];
        s.status_flags = b[31];
        s.sub_phase = b[32];
        s.active_worker = b[33];
        s.prev_square = b[34];
        s.current_player = b[35];
        s.game_over = b[36] != 0;
        s.winner = b[37] as i8;
        s.turn_count = u16::from_le_bytes([b[38], b[39]]);
        s.power_used = b[40];
        s.turn_flags = b[41];
        s.stage = b[42];
        s.tags = [[b[43], b[44]], [b[45], b[46]]];
        s.low_domes = u32::from_le_bytes([b[47], b[48], b[49], b[50]]);
        s
    }

    /// Validates every structural invariant on a deserialised state before trusting external
    /// bytes/hex (so untrusted Position Lab imports cannot violate search/movegen assumptions).
    pub fn validate_deserialized(&self) -> Result<(), &'static str> {
        use crate::gods::{
            is_supported_god, stage_binds_worker, CHAIN_MAX_STEPS, NUM_STAGES, TAG_APHRODITE, TAG_BUILT, TAG_FEMALE,
            TAG_FROZEN, TAG_MOVED,
        };
        const ALL_TAGS: u8 = TAG_FEMALE | TAG_APHRODITE | TAG_FROZEN | TAG_MOVED | TAG_BUILT;
        if self.height.iter().any(|&h| h > DOME) {
            return Err("square height out of range");
        }
        if self.current_player > 1 {
            return Err("current_player must be 0 or 1");
        }
        if !((self.game_over && (self.winner == 0 || self.winner == 1)) || (!self.game_over && self.winner == -1)) {
            return Err("inconsistent game_over / winner");
        }
        if !is_supported_god(self.gods[0]) || !is_supported_god(self.gods[1]) {
            return Err("unsupported god id");
        }
        if self.sub_phase > PHASE_SUB_TURN || (self.stage as usize) >= NUM_STAGES {
            return Err("invalid sub_phase or stage");
        }
        if (self.sub_phase == PHASE_SUB_TURN) != (self.stage != 0) {
            return Err("sub_phase and stage disagree");
        }
        if self.active_worker > 1 || (self.prev_square != UNPLACED && (self.prev_square as usize) >= NUM_SQUARES) {
            return Err("invalid active_worker or prev_square");
        }
        if self.power_used > 0b11
            || self.steps() > CHAIN_MAX_STEPS
            || (self.status_flags & !(0b11 | (0b111 << 4))) != 0
            || (self.turn_flags & !0b11_1111) != 0
        {
            return Err("invalid status/turn/power flags");
        }
        let mut occ = 0u32;
        for p in 0..2 {
            if self.workers[p][0] > self.workers[p][1] {
                return Err("workers are not canonically sorted");
            }
            for i in 0..2 {
                let w = self.workers[p][i];
                let t = self.tags[p][i];
                if w == UNPLACED {
                    if t != 0 {
                        return Err("unplaced worker carries tags");
                    }
                } else {
                    if (w as usize) >= NUM_SQUARES || self.height[w as usize] >= DOME {
                        return Err("worker on invalid or domed square");
                    }
                    if occ & (1 << w) != 0 {
                        return Err("duplicate worker square");
                    }
                    occ |= 1 << w;
                    if (t & !ALL_TAGS) != 0 {
                        return Err("invalid worker tag bits");
                    }
                }
            }
            if self.sub_phase >= PHASE_TURN && !self.game_over && self.workers[p][0] == UNPLACED {
                return Err("in-progress turn requires at least one worker per player");
            }
        }
        if stage_binds_worker(self.stage)
            && !self.game_over
            && self.workers[self.current_player as usize][self.active_worker as usize] == UNPLACED
        {
            return Err("stage binds an unplaced worker");
        }
        if self.low_domes >= (1 << NUM_SQUARES) {
            return Err("low_domes mask out of range");
        }
        let mut low = self.low_domes;
        while low != 0 {
            let sq = low.trailing_zeros() as usize;
            if self.height[sq] != DOME {
                return Err("low_domes bit set on non-dome square");
            }
            low &= low - 1;
        }
        Ok(())
    }

    /// Writes the side-to-move-relative observation (see `OBS_*`) into `out[..OBS_LEN]`.
    pub fn write_observation_into(&self, out: &mut [f32]) {
        use crate::gods::{
            stage_binds_worker, CHAIN_MAX_STEPS, TAG_APHRODITE, TAG_BUILT, TAG_FEMALE, TAG_FROZEN, TAG_MOVED, TF_DOWN,
            TF_PERSEPHONE,
        };
        let out = &mut out[..OBS_LEN];
        out.fill(0.0);
        for (sq, &h) in self.height.iter().enumerate() {
            out[h as usize * NUM_SQUARES + sq] = 1.0;
        }
        let (me, opp) = (self.current_player as usize, 1 - self.current_player as usize);
        for (k, p) in [me, opp].into_iter().enumerate() {
            for (i, &w) in self.workers[p].iter().enumerate() {
                if w != UNPLACED {
                    let sq = w as usize;
                    out[(OBS_PLANE_MY_WORKERS + k) * NUM_SQUARES + sq] = 1.0;
                    if self.tags[p][i] & TAG_FEMALE != 0 {
                        out[(OBS_PLANE_MY_FEMALE + k) * NUM_SQUARES + sq] = 1.0;
                    }
                    if k == 0 {
                        let t = self.tags[p][i];
                        for (bit, plane) in [
                            (TAG_FROZEN, OBS_PLANE_MY_FROZEN),
                            (TAG_APHRODITE, OBS_PLANE_MY_APHRODITE),
                            (TAG_MOVED, OBS_PLANE_MY_MOVED),
                            (TAG_BUILT, OBS_PLANE_MY_BUILT),
                        ] {
                            if t & bit != 0 {
                                out[plane * NUM_SQUARES + sq] = 1.0;
                            }
                        }
                    }
                }
                out[OBS_SPATIAL + 2 * k + i] = if w == UNPLACED { NUM_SQUARES as f32 } else { w as f32 };
            }
            out[OBS_SPATIAL + 4 + k] = crate::gods::god_slot(self.gods[p]) as f32;
        }
        let mut low = self.low_domes;
        while low != 0 {
            out[OBS_PLANE_LOW_DOMES * NUM_SQUARES + low.trailing_zeros() as usize] = 1.0;
            low &= low - 1;
        }
        if stage_binds_worker(self.stage) {
            let w = self.workers[me][self.active_worker as usize];
            if w != UNPLACED {
                out[OBS_PLANE_ACTIVE * NUM_SQUARES + w as usize] = 1.0;
            }
        }
        if self.prev_square != UNPLACED {
            out[OBS_PLANE_PREV * NUM_SQUARES + self.prev_square as usize] = 1.0;
        }
        for (plane, active) in [
            (OBS_PLANE_SETUP, self.sub_phase < PHASE_TURN),
            (OBS_PLANE_ME_RESTRICTED, self.athena_restricts(me)),
            (OBS_PLANE_OPP_RESTRICTED, self.athena_restricts(opp)),
            (OBS_PLANE_ME_POWER_USED, self.power_used & (1 << me) != 0),
            (OBS_PLANE_OPP_POWER_USED, self.power_used & (1 << opp) != 0),
            (OBS_PLANE_MOVED_UP, self.turn_flags & (1 << me) != 0),
            (OBS_PLANE_MOVED_DOWN, self.turn_flags & (1 << (TF_DOWN + me)) != 0),
            (OBS_PLANE_PERSEPHONE, self.turn_flags & (1 << (TF_PERSEPHONE + me)) != 0),
        ] {
            if active {
                out[plane * NUM_SQUARES..(plane + 1) * NUM_SQUARES].fill(1.0);
            }
        }
        if self.stage != 0 {
            let p = OBS_PLANE_STAGE_BASE + (self.stage - 1) as usize;
            out[p * NUM_SQUARES..(p + 1) * NUM_SQUARES].fill(1.0);
        }
        let steps = self.steps();
        if steps != 0 {
            let v = steps as f32 / CHAIN_MAX_STEPS as f32;
            out[OBS_PLANE_STEPS * NUM_SQUARES..(OBS_PLANE_STEPS + 1) * NUM_SQUARES].fill(v);
        }
    }

    /// Applies a legal action. Behaviour on an illegal action is unspecified (checked
    /// with `debug_assert!` only -- validate at the FFI boundary, not in the search).
    pub fn apply_action(&mut self, action: i32) {
        debug_assert!(!self.game_over, "apply_action on terminal state");
        let p = self.current_player as usize;
        // Saturate rather than overflow/wrap: Pantheon gods without mandatory net builds
        // (Castor & Pollux double-moves, Ares block removal) can step past u16::MAX if a
        // caller omits cycle/ply adjudication, while `game_over` stays purely positional.
        self.turn_count = self.turn_count.saturating_add(1);

        if self.sub_phase < PHASE_TURN {
            debug_assert!((0..NUM_SQUARES as i32).contains(&action) && self.occupied_mask() & (1 << action) == 0);
            self.workers[p][self.sub_phase as usize] = action as u8;
            if self.sub_phase == PHASE_PLACE_0 && matches!(self.gods[p], crate::gods::GOD_SELENE | crate::gods::GOD_HIPPOLYTA) {
                self.tags[p][0] |= crate::gods::TAG_FEMALE; // engine convention: first placed
            }
            self.sort_workers();
            if self.sub_phase == PHASE_PLACE_0 {
                self.sub_phase = PHASE_PLACE_1;
            } else if p as u8 == self.first_placer() {
                self.sub_phase = PHASE_PLACE_0;
                self.current_player ^= 1;
            } else {
                // Player 0 always moves first (even when Bia made player 1 place first).
                self.sub_phase = PHASE_TURN;
                self.current_player = 0;
                if !self.is_mortal_game() {
                    self.begin_turn(0);
                    // God rules can leave the first mover without a legal turn (e.g.
                    // Terpsichore boxed in by Limus): that is a loss, as on any later turn.
                    if !self.has_valid_action() {
                        self.game_over = true;
                        self.winner = 1;
                    }
                }
            }
            return;
        }

        if !self.is_mortal_game() {
            return self.apply_action_gods(action);
        }
        let (w, m, b) = decode_turn_action(action);
        let from = self.workers[p][w] as usize;
        let to = NEIGHBORS[from][m];
        debug_assert!(to != OFF_BOARD);
        self.workers[p][w] = to;
        if self.height[to as usize] == 3 && self.height[from] == 2 {
            self.sort_workers();
            self.game_over = true;
            self.winner = p as i8;
            return;
        }
        let target = NEIGHBORS[to as usize][b];
        debug_assert!(target != OFF_BOARD && self.height[target as usize] < DOME);
        self.height[target as usize] += 1;
        self.sort_workers();
        self.current_player ^= 1;
        if !self.has_valid_action() {
            // A player who cannot complete a move + build loses (2-player rules).
            self.game_over = true;
            self.winner = p as i8;
        }
    }

    /// Bit-exact Zobrist hash of the position (excludes `turn_count`, so transpositions
    /// collide). Worker keys are XORed per player, which makes the hash invariant to
    /// worker order by construction -- sorting is still kept for action indexing and
    /// `==`. `game_over`/`winner` are omitted: they are functions of the position.
    #[inline]
    pub fn state_hash(&self) -> u64 {
        let mut h = 0u64;
        for (sq, &ht) in self.height.iter().enumerate() {
            if ht != 0 {
                h ^= ZOBRIST[Z_HEIGHT + sq * 5 + ht as usize];
            }
        }
        for p in 0..2 {
            for &w in &self.workers[p] {
                if w != UNPLACED {
                    h ^= ZOBRIST[Z_WORKER + p * NUM_SQUARES + w as usize];
                }
            }
            let g = self.gods[p] as usize;
            if g != 0 {
                h ^= ZOBRIST[Z_GOD + p * crate::gods::NUM_GODS + g];
            }
            if self.power_used & (1 << p) != 0 {
                h ^= ZOBRIST[Z_POWER + p];
            }
            for i in 0..2 {
                let (w, t) = (self.workers[p][i], self.tags[p][i]);
                if t != 0 && w != UNPLACED {
                    for bit in 0..8 {
                        if t & (1 << bit) != 0 {
                            h ^= ZOBRIST[Z_TAG + (p * NUM_SQUARES + w as usize) * 8 + bit];
                        }
                    }
                }
            }
        }
        if self.current_player == 1 {
            h ^= ZOBRIST[Z_SIDE];
        }
        h ^= ZOBRIST[Z_PHASE + self.sub_phase as usize];
        for bit in 0..8 {
            if self.status_flags & (1 << bit) != 0 {
                h ^= ZOBRIST[Z_FLAG + bit];
            }
        }
        if crate::gods::stage_binds_worker(self.stage) {
            h ^= ZOBRIST[Z_ACTIVE + self.active_worker as usize];
        }
        if self.prev_square != UNPLACED {
            h ^= ZOBRIST[Z_PREV + self.prev_square as usize];
        }
        for bit in 0..8 {
            if self.turn_flags & (1 << bit) != 0 {
                h ^= ZOBRIST[Z_TURN_FLAG + bit];
            }
        }
        if self.stage != 0 {
            h ^= ZOBRIST[Z_STAGE + (self.stage - 1) as usize];
        }
        let mut low = self.low_domes;
        while low != 0 {
            h ^= ZOBRIST[Z_LOW_DOME + low.trailing_zeros() as usize];
            low &= low - 1;
        }
        h
    }

    /// Zobrist hash of `child = self.apply_action(action)` given `parent_hash =
    /// self.state_hash()`, updated incrementally for ordinary turns (worker move, one
    /// build, side flip) and recomputed for setup / terminal children. Bit-identical to
    /// `child.state_hash()` (tested); saves a ~50 ns full rehash per search node.
    #[inline]
    pub fn child_hash(&self, parent_hash: u64, action: i32, child: &Self) -> u64 {
        if self.sub_phase != PHASE_TURN
            || child.sub_phase != PHASE_TURN
            || child.game_over
            || !self.is_mortal_game()
            || self.status_flags != child.status_flags
        {
            return child.state_hash();
        }
        let p = self.current_player as usize;
        let (w, m, b) = decode_turn_action(action);
        let from = self.workers[p][w] as usize;
        let to = NEIGHBORS[from][m] as usize;
        let bsq = NEIGHBORS[to][b] as usize;
        let mut h = parent_hash ^ ZOBRIST[Z_WORKER + p * NUM_SQUARES + from] ^ ZOBRIST[Z_WORKER + p * NUM_SQUARES + to];
        let (old, new) = (self.height[bsq] as usize, child.height[bsq] as usize);
        if old != 0 {
            h ^= ZOBRIST[Z_HEIGHT + bsq * 5 + old];
        }
        h ^= ZOBRIST[Z_HEIGHT + bsq * 5 + new];
        h ^ ZOBRIST[Z_SIDE]
    }

    /// Hash of this position with the side to move flipped (null move).
    #[inline(always)]
    pub fn null_move_hash(parent_hash: u64) -> u64 {
        parent_hash ^ ZOBRIST[Z_SIDE]
    }

    /// Image of this state under D4 symmetry `sym` (workers re-sorted).
    pub fn transform(&self, sym: usize) -> Self {
        let perm = &PERMUTE_SQ[sym];
        let mut t = *self;
        for sq in 0..NUM_SQUARES {
            t.height[perm[sq] as usize] = self.height[sq];
        }
        for p in 0..2 {
            for i in 0..2 {
                let w = self.workers[p][i];
                t.workers[p][i] = if w == UNPLACED { UNPLACED } else { perm[w as usize] };
            }
        }
        if self.prev_square != UNPLACED {
            t.prev_square = perm[self.prev_square as usize];
        }
        t.low_domes = 0;
        for sq in 0..NUM_SQUARES {
            if self.low_domes & (1 << sq) != 0 {
                t.low_domes |= 1 << perm[sq];
            }
        }
        let cp = self.current_player as usize;
        let active_sq = t.workers[cp][self.active_worker as usize & 1];
        t.sort_workers();
        if crate::gods::stage_binds_worker(self.stage) {
            // The sort may swap the pair: keep `active_worker` pointing at the same worker.
            t.active_worker = t.workers[cp].iter().position(|&x| x == active_sq).unwrap_or(0) as u8;
        }
        t
    }

    /// Image of `action` (legal in `self`) under `sym`, legal in `self.transform(sym)`.
    /// Needs the source state because a symmetry can flip the sorted worker order.
    #[inline]
    pub fn transform_action(&self, action: i32, sym: usize) -> i32 {
        use crate::gods::*;
        let ws = self.workers[self.current_player as usize];
        // A lone worker (the other removed) stays index 0.
        let new_w = |w: usize| {
            (ws[1 - w] != UNPLACED && PERMUTE_SQ[sym][ws[w] as usize] > PERMUTE_SQ[sym][ws[1 - w] as usize]) as usize
        };
        let d = |x: usize| PERMUTE_DIR[sym][x] as usize;
        match decode_action(action) {
            ActionKind::Place(sq) => PERMUTE_SQ[sym][sq] as i32,
            ActionKind::Turn { w, m, b, dome: false } => encode_turn_action(new_w(w), d(m), d(b)),
            ActionKind::Turn { w, m, b, dome: true } => encode_dome_turn(new_w(w), d(m), d(b)),
            ActionKind::MoveOnly { w, m } => encode_move_only(new_w(w), d(m)),
            ActionKind::BuildOnly { w, d: dir } => encode_build_only(new_w(w), d(dir)),
            ActionKind::Pass => action,
            ActionKind::Square(sq) => encode_square(PERMUTE_SQ[sym][sq] as usize),
            ActionKind::DomeOnly { w, d: dir } => encode_dome_only(new_w(w), d(dir)),
            ActionKind::Zeus { w, m } => encode_zeus(new_w(w), d(m)),
            ActionKind::Force { w, d: dir } => encode_force(new_w(w), d(dir)),
        }
    }
}
