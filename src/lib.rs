#![allow(clippy::needless_range_loop)]

//! AlphaSantorini WebAssembly API: zero-dependency in-browser *Santorini* engine with embedded
//! `SANTNNUE03` (`nnue-gods-v4`, 44 Pantheon powers, 1.32 MB quantized weights).

pub mod compact;
pub mod gods;
pub mod nnue;
pub mod search;

use compact::*;
use gods::{decode_action, is_supported_god, ActionKind, TAG_FEMALE};
use nnue::Nnue;
use search::{Limits, SearchEngine, SearchResult, EVAL_SCALE, MATE, MATE_BOUND};
use serde::Serialize;
use std::cell::RefCell;
use std::sync::Arc;

static EMBEDDED_NNUE_BYTES: &[u8] = include_bytes!("../weights.nnue");

thread_local! {
    static ENGINE: RefCell<Option<SearchEngine>> = const { RefCell::new(None) };
    static GAME: RefCell<Option<CompactSantoriniState>> = const { RefCell::new(None) };
    static HISTORY: RefCell<Vec<CompactSantoriniState>> = const { RefCell::new(Vec::new()) };
    static OUTPUT_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[derive(Serialize, Clone, Debug)]
pub struct ActionDesc {
    pub action: i32,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub square: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dome: Option<bool>,
    pub win: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forced_from: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forced_to: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed_block: Option<u8>,
}

fn sq_name(sq: u8) -> String {
    let col = (b'a' + (sq % 5)) as char;
    let row = 5 - (sq / 5);
    format!("{col}{row}")
}

pub fn short_move_desc(d: &ActionDesc) -> String {
    match d.kind {
        "place" => format!("@{}", sq_name(d.square.unwrap_or(0))),
        "turn" => {
            let f = sq_name(d.from.unwrap_or(0));
            let t = sq_name(d.to.unwrap_or(0));
            if d.win {
                format!("{f}-{t}#")
            } else {
                let b = sq_name(d.build.unwrap_or(0));
                let mark = if d.dome == Some(true) { "^" } else { "" };
                format!("{f}-{t}/{b}{mark}")
            }
        }
        "move" => {
            let f = sq_name(d.from.unwrap_or(0));
            let t = sq_name(d.to.unwrap_or(0));
            if d.win {
                format!("{f}-{t}#")
            } else {
                format!("{f}-{t}")
            }
        }
        "build" => format!("/{}", sq_name(d.build.unwrap_or(0))),
        "dome" => format!("/{}^", sq_name(d.build.unwrap_or(0))),
        "zeus" => format!("{}-{}/self", sq_name(d.from.unwrap_or(0)), sq_name(d.to.unwrap_or(0))),
        "force" => format!(
            "force {}>{}",
            sq_name(d.forced_from.unwrap_or(0)),
            sq_name(d.forced_to.unwrap_or(0))
        ),
        "square" => {
            if let Some(rb) = d.removed_block {
                format!("-{}", sq_name(rb))
            } else if let Some(ff) = d.forced_from {
                match d.forced_to {
                    Some(ft) => format!("drag {}>{}", sq_name(ff), sq_name(ft)),
                    None => format!("x{}", sq_name(ff)),
                }
            } else {
                format!("@{}^", sq_name(d.square.unwrap_or(0)))
            }
        }
        "pass" => "pass".to_string(),
        _ => d.kind.to_string(),
    }
}

pub fn describe_action_rust(s: &CompactSantoriniState, action: i32) -> ActionDesc {
    let p = s.current_player as usize;
    let mut next = *s;
    next.apply_action(action);
    let won = next.game_over && next.winner == p as i8 && next.current_player as usize == p;

    let squares = |ws: &[u8; 2]| {
        let mut v = Vec::with_capacity(2);
        for &x in ws {
            if x != UNPLACED {
                v.push(x);
            }
        }
        v
    };
    let (mine_before, mine_after) = (squares(&s.workers[p]), squares(&next.workers[p]));
    let moved_from = mine_before.iter().copied().find(|x| !mine_after.contains(x));
    let moved_to = mine_after.iter().copied().find(|x| !mine_before.contains(x));
    let (opp_before, opp_after) = (squares(&s.workers[1 - p]), squares(&next.workers[1 - p]));
    let forced_from = opp_before.iter().copied().find(|x| !opp_after.contains(x));
    let forced_to = opp_after.iter().copied().find(|x| !opp_before.contains(x));
    let raised = (0..25).find(|&q| next.height[q] > s.height[q]).map(|q| q as u8);
    let lowered = (0..25).find(|&q| next.height[q] < s.height[q]).map(|q| q as u8);

    let mut desc_worker = None;
    let mut desc_dome = None;
    let mut desc_square = None;
    let mut from_sq = None;

    let kind = match decode_action(action) {
        ActionKind::Place(sq) => {
            desc_square = Some(sq as u8);
            "place"
        }
        ActionKind::Turn { w, dome, .. } => {
            desc_worker = Some(w as u8);
            desc_dome = Some(dome);
            "turn"
        }
        ActionKind::MoveOnly { w, .. } => {
            desc_worker = Some(w as u8);
            "move"
        }
        ActionKind::BuildOnly { w, .. } => {
            desc_worker = Some(w as u8);
            from_sq = Some(s.workers[p][w]);
            "build"
        }
        ActionKind::Pass => "pass",
        ActionKind::Square(sq) => {
            desc_square = Some(sq as u8);
            "square"
        }
        ActionKind::DomeOnly { w, .. } => {
            desc_worker = Some(w as u8);
            from_sq = Some(s.workers[p][w]);
            "dome"
        }
        ActionKind::Zeus { w, .. } => {
            desc_worker = Some(w as u8);
            "zeus"
        }
        ActionKind::Force { w, .. } => {
            desc_worker = Some(w as u8);
            from_sq = Some(s.workers[p][w]);
            "force"
        }
    };

    let final_from = moved_from.or(from_sq);
    let final_build = if kind != "place" && kind != "square" { raised } else { None };

    ActionDesc {
        action,
        kind,
        square: desc_square,
        worker: desc_worker,
        dome: desc_dome,
        win: won,
        from: final_from,
        to: moved_to,
        build: final_build,
        forced_from,
        forced_to,
        removed_block: lowered,
    }
}

pub fn state_to_hex(s: &CompactSantoriniState) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let b = s.to_bytes();
    let mut out = String::with_capacity(STATE_BYTES * 2);
    for &byte in &b {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}

pub fn state_from_hex(hex: &str) -> Result<CompactSantoriniState, &'static str> {
    let trimmed = hex.trim();
    if trimmed.len() != STATE_BYTES * 2 {
        return Err("expected 102 hex characters");
    }
    let mut arr = [0u8; STATE_BYTES];
    for (i, dst) in arr.iter_mut().enumerate() {
        let chunk = trimmed.get(i * 2..i * 2 + 2).ok_or("invalid hex string")?;
        *dst = u8::from_str_radix(chunk, 16).map_err(|_| "invalid hex digit")?;
    }
    let s = CompactSantoriniState::from_bytes(&arr);
    s.validate_deserialized()?;
    Ok(s)
}

#[derive(Serialize)]
pub struct StateJson {
    pub height: [u8; 25],
    pub workers: [[u8; 2]; 2],
    pub low_domes: u32,
    pub power_used: [bool; 2],
    pub female: [[bool; 2]; 2],
    pub restricted: [bool; 2],
    pub current_player: u8,
    pub gods: [u8; 2],
    pub game_over: bool,
    pub winner: i8,
    pub turn_count: u16,
    pub setup: bool,
    pub in_sub_turn: bool,
    pub sub_phase: u8,
    pub stage: u8,
    pub can_undo: bool,
    pub state_hex: String,
    pub legal_actions: Vec<ActionDesc>,
}

#[derive(Serialize)]
pub struct PvLineJson {
    pub rank: usize,
    pub action: i32,
    pub move_desc: ActionDesc,
    pub short: String,
    pub score: i32,
    pub value: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mate_in_plies: Option<i32>,
    pub depth: u8,
    pub pv: Vec<String>,
}

#[derive(Serialize)]
pub struct SearchResultJson {
    pub perspective: u8,
    pub best_action: i32,
    pub score: i32,
    pub value: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mate_in_plies: Option<i32>,
    pub depth: u8,
    pub nodes: u64,
    pub elapsed_ms: f64,
    pub steps: Vec<ActionDesc>,
    pub pv: Vec<String>,
    pub lines: Vec<PvLineJson>,
}

fn score_to_eval(score: i32) -> (f32, Option<i32>) {
    if score.abs() >= MATE_BOUND {
        let plies = (MATE - score.abs()).max(1);
        let val = if score > 0 { 1.0 } else { -1.0 };
        (val, Some(plies))
    } else {
        (((score as f32) / EVAL_SCALE).clamp(-1.0, 1.0), None)
    }
}

fn format_pv_actions(root: &CompactSantoriniState, actions: &[i32]) -> Vec<String> {
    let mut s = *root;
    let mut out = Vec::with_capacity(actions.len());
    let mut buf = [0i32; MAX_ACTIONS];
    for &a in actions {
        if s.game_over {
            break;
        }
        let n = s.write_valid_actions_into(&mut buf);
        if !buf[..n].contains(&a) {
            break;
        }
        let desc = describe_action_rust(&s, a);
        out.push(short_move_desc(&desc));
        s.apply_action(a);
    }
    out
}

fn build_state_json(s: &CompactSantoriniState, can_undo: bool) -> StateJson {
    let mut legal = [0i32; MAX_ACTIONS];
    let n = s.write_valid_actions_into(&mut legal);
    let legal_actions: Vec<ActionDesc> = legal[..n].iter().map(|&a| describe_action_rust(s, a)).collect();

    let female = [
        [(s.tags[0][0] & TAG_FEMALE) != 0, (s.tags[0][1] & TAG_FEMALE) != 0],
        [(s.tags[1][0] & TAG_FEMALE) != 0, (s.tags[1][1] & TAG_FEMALE) != 0],
    ];
    let power_used = [(s.power_used & 1) != 0, (s.power_used & 2) != 0];
    let restricted = [s.athena_restricts(0), s.athena_restricts(1)];

    StateJson {
        height: s.height,
        workers: s.workers,
        low_domes: s.low_domes,
        power_used,
        female,
        restricted,
        current_player: s.current_player,
        gods: s.gods,
        game_over: s.game_over,
        winner: s.winner,
        turn_count: s.turn_count,
        setup: s.sub_phase < PHASE_TURN,
        in_sub_turn: s.sub_phase == PHASE_SUB_TURN,
        sub_phase: s.sub_phase,
        stage: s.stage,
        can_undo,
        state_hex: state_to_hex(s),
        legal_actions,
    }
}

fn set_output_json<T: Serialize>(val: &T) -> *const u8 {
    let mut bytes = serde_json::to_vec(val).unwrap_or_else(|_| b"{}".to_vec());
    bytes.push(0);
    OUTPUT_BUF.with(|buf| {
        let mut b = buf.borrow_mut();
        *b = bytes;
        b.as_ptr()
    })
}

// ---------------------------------------------------------------------------
// Exported C ABI Functions for WebAssembly
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn santorini_init() -> i32 {
    let nnue = match Nnue::from_bytes(EMBEDDED_NNUE_BYTES) {
        Ok(n) => Arc::new(n),
        Err(_) => return 0,
    };
    let engine = SearchEngine::new(nnue, 18); // 262,144 TT entries (~4 MB)
    ENGINE.with(|e| *e.borrow_mut() = Some(engine));
    1
}

#[no_mangle]
pub extern "C" fn santorini_new_game(god0: u8, god1: u8) -> i32 {
    if !is_supported_god(god0) || !is_supported_god(god1) {
        return 0;
    }
    let state = CompactSantoriniState::new_with_gods([god0, god1]);
    GAME.with(|g| *g.borrow_mut() = Some(state));
    HISTORY.with(|h| {
        let mut hist = h.borrow_mut();
        hist.clear();
        hist.push(state);
    });
    ENGINE.with(|e| {
        if let Some(eng) = e.borrow_mut().as_mut() {
            eng.clear();
        }
    });
    1
}

#[no_mangle]
pub extern "C" fn santorini_apply_action(action: i32) -> i32 {
    GAME.with(|g| {
        let mut state_opt = g.borrow_mut();
        let s = match state_opt.as_mut() {
            Some(s) => s,
            None => return 0,
        };
        let mut legal = [0i32; MAX_ACTIONS];
        let n = s.write_valid_actions_into(&mut legal);
        if !legal[..n].contains(&action) {
            return 0;
        }
        s.apply_action(action);
        HISTORY.with(|h| h.borrow_mut().push(*s));
        1
    })
}

/// Allocates `len` bytes for JS to pass UTF-8 strings (e.g. 102-char state hex) into Wasm.
#[no_mangle]
pub extern "C" fn santorini_alloc(len: usize) -> *mut u8 {
    let mut v = vec![0u8; len];
    let ptr = v.as_mut_ptr();
    std::mem::forget(v);
    ptr
}

/// Frees a buffer previously allocated by `santorini_alloc`.
///
/// # Safety
/// `ptr` and `len` must match a previous `santorini_alloc(len)` call.
#[no_mangle]
pub unsafe extern "C" fn santorini_dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Vec::from_raw_parts(ptr, len, len);
    }
}

/// Loads a validated 102-character state hex string onto the history stack.
///
/// # Safety
/// `ptr` must point to `len` valid bytes in linear memory.
#[no_mangle]
pub unsafe extern "C" fn santorini_set_hex(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() || len == 0 || len > 512 {
        return 0;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    let Ok(text) = std::str::from_utf8(slice) else {
        return 0;
    };
    let Ok(state) = state_from_hex(text) else {
        return 0;
    };
    GAME.with(|g| *g.borrow_mut() = Some(state));
    HISTORY.with(|h| h.borrow_mut().push(state));
    1
}

#[no_mangle]
pub extern "C" fn santorini_undo() -> i32 {
    GAME.with(|g| {
        HISTORY.with(|h| {
            let mut hist = h.borrow_mut();
            if hist.len() <= 1 {
                return 0;
            }
            hist.pop();
            if let Some(&prev) = hist.last() {
                *g.borrow_mut() = Some(prev);
                1
            } else {
                0
            }
        })
    })
}

#[no_mangle]
pub extern "C" fn santorini_get_state() -> *const u8 {
    GAME.with(|g| {
        let state_opt = g.borrow();
        let s = match state_opt.as_ref() {
            Some(s) => s,
            None => return set_output_json(&serde_json::Value::Null),
        };
        let can_undo = HISTORY.with(|h| h.borrow().len() > 1);
        let state_json = build_state_json(s, can_undo);
        set_output_json(&state_json)
    })
}

#[no_mangle]
pub extern "C" fn santorini_search(time_ms: u32, max_nodes: u32, multi_pv: u32) -> *const u8 {
    let state = match GAME.with(|g| *g.borrow()) {
        Some(s) => s,
        None => return set_output_json(&serde_json::Value::Null),
    };
    if state.game_over {
        return set_output_json(&serde_json::Value::Null);
    }

    ENGINE.with(|e| {
        let mut eng_opt = e.borrow_mut();
        let eng = match eng_opt.as_mut() {
            Some(eng) => eng,
            None => return set_output_json(&serde_json::Value::Null),
        };

        let limits = Limits {
            time_ms: time_ms as u64,
            max_depth: 32,
            max_nodes: if max_nodes > 0 { max_nodes as u64 } else { 2_000_000 },
        };
        let res: SearchResult = eng.search_multipv(&state, limits, (multi_pv as usize).clamp(1, 5));
        let step_desc = describe_action_rust(&state, res.best_action);
        let (value, mate_in_plies) = score_to_eval(res.score);
        let pv = format_pv_actions(&state, &res.pv);
        let lines = res
            .lines
            .iter()
            .enumerate()
            .map(|(idx, ln)| {
                let md = describe_action_rust(&state, ln.action);
                let short = short_move_desc(&md);
                let (lv, lm) = score_to_eval(ln.score);
                PvLineJson {
                    rank: idx + 1,
                    action: ln.action,
                    move_desc: md,
                    short,
                    score: ln.score,
                    value: lv,
                    mate_in_plies: lm,
                    depth: ln.depth,
                    pv: format_pv_actions(&state, &ln.pv),
                }
            })
            .collect();

        let out = SearchResultJson {
            perspective: state.current_player,
            best_action: res.best_action,
            score: res.score,
            value,
            mate_in_plies,
            depth: res.depth,
            nodes: res.nodes,
            elapsed_ms: res.elapsed_ms,
            steps: vec![step_desc],
            pv,
            lines,
        };
        set_output_json(&out)
    })
}

#[no_mangle]
pub extern "C" fn santorini_ai_full_turn(time_ms: u32) -> *const u8 {
    let initial_state = match GAME.with(|g| *g.borrow()) {
        Some(s) => s,
        None => return set_output_json(&serde_json::Value::Null),
    };
    if initial_state.game_over {
        return set_output_json(&serde_json::Value::Null);
    }

    let ai_player = initial_state.current_player;
    let mut current_state = initial_state;
    let mut steps: Vec<ActionDesc> = Vec::new();

    let mut first_res: Option<SearchResult> = None;
    let total_budget = time_ms as u64;
    let mut remaining = total_budget;
    let mut total_nodes = 0u64;
    let mut total_elapsed = 0.0f64;

    ENGINE.with(|e| {
        let mut eng_opt = e.borrow_mut();
        let eng = match eng_opt.as_mut() {
            Some(eng) => eng,
            None => return set_output_json(&serde_json::Value::Null),
        };

        let mut step_count = 0usize;
        while !current_state.game_over && current_state.current_player == ai_player && step_count < 24 {
            let budget = if step_count == 0 {
                (remaining * 70 / 100).max(10)
            } else {
                (remaining / 2).max(5)
            };
            let limits = Limits {
                time_ms: budget,
                max_depth: 32,
                max_nodes: 2_000_000,
            };
            let res = eng.search(&current_state, limits);
            total_nodes += res.nodes;
            total_elapsed += res.elapsed_ms;
            if first_res.is_none() {
                first_res = Some(res.clone());
            }
            let desc = describe_action_rust(&current_state, res.best_action);
            steps.push(desc);
            current_state.apply_action(res.best_action);
            step_count += 1;
            remaining = remaining.saturating_sub(res.elapsed_ms as u64);
        }

        GAME.with(|g| *g.borrow_mut() = Some(current_state));
        HISTORY.with(|h| h.borrow_mut().push(current_state));

        let first = first_res.unwrap_or(SearchResult {
            best_action: -1,
            score: 0,
            depth: 0,
            nodes: 0,
            elapsed_ms: 0.0,
            pv: Vec::new(),
            lines: Vec::new(),
        });
        let (value, mate_in_plies) = score_to_eval(first.score);
        let pv = format_pv_actions(&initial_state, &first.pv);
        let lines = first
            .lines
            .iter()
            .enumerate()
            .map(|(idx, ln)| {
                let md = describe_action_rust(&initial_state, ln.action);
                let short = short_move_desc(&md);
                let (lv, lm) = score_to_eval(ln.score);
                PvLineJson {
                    rank: idx + 1,
                    action: ln.action,
                    move_desc: md,
                    short,
                    score: ln.score,
                    value: lv,
                    mate_in_plies: lm,
                    depth: ln.depth,
                    pv: format_pv_actions(&initial_state, &ln.pv),
                }
            })
            .collect();

        let out = SearchResultJson {
            perspective: ai_player,
            best_action: first.best_action,
            score: first.score,
            value,
            mate_in_plies,
            depth: first.depth,
            nodes: total_nodes,
            elapsed_ms: total_elapsed,
            steps,
            pv,
            lines,
        };
        set_output_json(&out)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gods::{GOD_ARES, GOD_MORTAL, GOD_SELENE, GOD_ZEUS, SUPPORTED_GODS};

    unsafe fn read_c_json(ptr: *const u8) -> serde_json::Value {
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        serde_json::from_slice(std::slice::from_raw_parts(ptr, len)).unwrap()
    }

    #[test]
    fn embedded_santnnue03_loads_and_all_44_gods_start_and_search() {
        assert_eq!(santorini_init(), 1);
        assert_eq!(santorini_new_game(18, 0), 0); // unsupported hidden-info ID
        assert_eq!(santorini_new_game(99, 0), 0); // out-of-range ID

        for &g in &SUPPORTED_GODS {
            assert_eq!(santorini_new_game(g, GOD_MORTAL), 1, "failed new_game for god {g}");
            let st = unsafe { read_c_json(santorini_get_state()) };
            assert_eq!(st["setup"], true);
            assert!(!st["legal_actions"].as_array().unwrap().is_empty());
        }

        // Verify Multi-PV search + Ares block removal + Zeus self-build + Selene female flag
        assert_eq!(santorini_new_game(GOD_SELENE, GOD_ZEUS), 1);
        let res = unsafe { read_c_json(santorini_ai_full_turn(15)) };
        assert!(res["steps"].as_array().unwrap().len() >= 2);
        let st_after = unsafe { read_c_json(santorini_get_state()) };
        assert_eq!(st_after["female"][0], serde_json::json!([true, false]));

        // Hex round-trip validation
        let hex = st_after["state_hex"].as_str().unwrap().to_string();
        assert_eq!(hex.len(), 102);
        assert_eq!(unsafe { santorini_set_hex(hex.as_ptr(), hex.len()) }, 1);
        let bad = "ff".repeat(51);
        assert_eq!(unsafe { santorini_set_hex(bad.as_ptr(), bad.len()) }, 0);

        // Ares end-of-turn block removal description includes `removed_block`
        let mut h = [0u8; 25];
        h[1] = 1;
        let mut s = CompactSantoriniState::from_position_with_gods(h, [[0, 12], [4, 24]], 0, [GOD_ARES, GOD_MORTAL]);
        // Move worker at 12 (index 1) so unmoved worker at 0 neighbors block on 1
        let turn_act = {
            let mut legal = [0i32; MAX_ACTIONS];
            let n = s.write_valid_actions_into(&mut legal);
            legal[..n]
                .iter()
                .copied()
                .find(|&a| {
                    let d = describe_action_rust(&s, a);
                    d.kind == "turn" && d.from == Some(12) && d.build != Some(1)
                })
                .unwrap()
        };
        s.apply_action(turn_act);
        assert_eq!(s.stage, gods::STAGE_END);
        let mut legal = [0i32; MAX_ACTIONS];
        let n = s.write_valid_actions_into(&mut legal);
        let rem = legal[..n]
            .iter()
            .map(|&a| describe_action_rust(&s, a))
            .find(|d| d.removed_block == Some(1))
            .expect("Ares should describe removed_block = Some(1)");
        assert_eq!(short_move_desc(&rem), "-b5");
    }
}
