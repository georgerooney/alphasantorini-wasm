//! God powers following **Santorini 2nd Edition / Pantheon** rules.
//!
//! # Intro gods (Phase 4; self-play trains on these)
//!
//! | ID | God | Power (as implemented) |
//! | :-: | :--- | :--- |
//! | 1 | Apollo | May move into an opponent's space **on the same or a lower level**, forcing them to the vacated space. (2E change: 1E also allowed swapping up.) |
//! | 2 | Artemis | May move one additional time, never back to the initial space. |
//! | 3 | Athena | If she moved up on her last turn, opponent workers cannot move up. |
//! | 4 | Atlas | May build a dome at any level. |
//! | 5 | Demeter | May build one additional time, not on the same space. |
//! | 6 | Hephaestus | May build one additional block (not a dome) on top of the first. |
//! | 7 | Hermes | If not moving up or down, workers may each move any number of times (even zero), then either builds. **Engine cap: `HERMES_MAX_STEPS` level steps per turn** so search trees stay finite. |
//! | 8 | Minotaur | May move into an opponent's space by forcing them one space further in the same direction, onto an unoccupied (no worker, no dome) space at any level. Forced workers never win. |
//! | 9 | Pan | Also wins by moving down two or more levels. |
//! | 10 | Prometheus | Optional turn: as long as the worker does not move up, it may build, move, then build again. |
//!
//! # Pantheon gods (rules + search only; the networks treat them as Mortal until trained)
//!
//! 2E rule mechanics; standard 2E timing applies (e.g. a win is any move *up*
//! onto level 3, checked before Harpies' slide). Engine conventions are marked **(engine)**.
//!
//! | ID | God | Power (as implemented) |
//! | :-: | :--- | :--- |
//! | 11 | Aphrodite | Opponent workers neighbouring hers at the start of their turn must neighbour one of hers at its end (even a winning end). |
//! | 12 | Ares | End of turn: may remove an unoccupied block (not dome) neighbouring the unmoved worker. |
//! | 13 | Bia | Places first. After a move, an opponent worker on the next space in the same direction is removed. |
//! | 15 | Charon | Instead of moving: force a neighbouring opponent worker to the unoccupied space directly behind your worker, then build. |
//! | 16 | Chronus | Also wins if at the end of their turn there are >= 4 complete towers (dome on level 3). |
//! | 19 | Eros | Places on opposite edges (first worker on the perimeter). Also wins if at the end of their turn both workers are on level 1 and neighbour each other. |
//! | 20 | Hera | An opponent cannot win by moving into a perimeter space. |
//! | 21 | Hestia | May build one additional time, not on a perimeter space. |
//! | 22 | Hypnus | At the start of an opponent's turn, their strictly highest worker cannot move. |
//! | 23 | Limus | Opponents cannot build on spaces neighbouring Limus workers, unless building a dome. |
//! | 24 | Medusa | End of turn: may remove an opponent worker that neighbours one of hers and is on a lower level; it becomes a dome. |
//! | 26 | Persephone | Opponents must move up at least once during their turn if any turn allows it. |
//! | 27 | Poseidon | End of turn: if the unmoved worker is on the ground, it may build up to two more times. |
//! | 28 | Selene | Instead of building, may build a dome at any level next to her female worker. **(engine) The first-placed worker is the female** (worker 0 in `from_position`). |
//! | 29 | Triton | Each move into a perimeter space may be followed by another move. **(engine) at most `CHAIN_MAX_STEPS` extra moves.** |
//! | 30 | Zeus | Instead of building, may build a block under the moved worker (never a win). |
//! | 37 | Hades | Opponent workers cannot move down. |
//! | 38 | Harpies | Each opponent move continues in the same direction until the next space is higher, a dome, occupied or off-board. |
//! | 44 | Terpsichore | Both workers must move, then both must build. |
//! | 45 | Urania | Moves and builds wrap across the board edges. |
//! | 46 | Achilles | Once: build before and after moving (with the same worker). |
//! | 48 | Atalanta | Once: the moved worker moves any number of extra times. **(engine) cap `CHAIN_MAX_STEPS`.** |
//! | 49 | Bellerophon | Once: may move up two levels (and win doing so). |
//! | 50 | Heracles | Once, instead of building: both workers build any number of domes on neighbouring spaces. |
//! | 54 | Polyphemus | Once, end of turn: build up to two domes on any unoccupied spaces. |
//! | 55 | Theseus | Once, end of turn: remove opponent workers neighbouring yours that are >= 2 levels higher. |
//! | 57 | Scylla | After moving, may drag an opponent worker neighbouring the vacated space into it. |
//! | 58 | Castor & Pollux | Alternative turn: move all workers without building, or build with all without moving. |
//! | 61 | Maenads | Wins if at the end of their turn two of their workers flank an opponent worker on opposite sides. |
//! | 62 | Asteria | End of turn: if a worker moved down this turn, may dome any unoccupied space. |
//! | 63 | Hippolyta | Her female worker moves normally; every other worker (both players) only diagonally. **(engine) first-placed = female.** |
//! | 65 | Iris | May move over a neighbouring worker to the unoccupied space behind it, at any level. |
//! | 67 | Pegasus | May move up more than one level, but cannot win by doing so. |
//!
//! Not implemented: hidden-information / dice / token gods (Hecate, Moerae, Tartarus, Nyx, Tyche,
//! Odysseus, Aeolus, Charybdis, Clio, Europa, Gaea, ...), Siren (her fixed compass direction breaks
//! the D4 symmetry the training augmentation relies on) and Adonis (ambiguous 2E text).
//!
//! # Turn structure
//! A god turn is one or more plies by the same player. Single-ply turns reuse the Base
//! encoding (`25 + w*64 + m*8 + b`) plus dome-turns. Multi-ply turns use `PHASE_SUB_TURN` with an
//! explicit non-zero `stage` (`STAGE_*`), plus `active_worker` (for worker-bound stages) and
//! `prev_square` where a restriction square applies.
//!
//! | Range | Meaning |
//! | :--- | :--- |
//! | `153..281` | dome-turn `(w, move_dir, build_dir)`: Atlas; Selene (`build_dir` from her female worker) |
//! | `281..297` | move-only step `(w, dir)` |
//! | `297..313` | build-only step `(w, dir)` |
//! | `313` | pass (end an optional step) |
//! | `314..339` | square target (end-of-turn powers, Scylla) |
//! | `339..355` | dome-only build `(w, dir)` (Heracles) |
//! | `355..371` | Zeus: move `(w, dir)` then build under the worker |
//! | `371..387` | Charon: force the worker at `(w, dir)` behind `w` |
//!
//! Every action is only legal if the turn can still be completed, so "no legal action" keeps
//! meaning "this player has lost". Pantheon games enforce that generically: each candidate is
//! applied and a continuation (or the end-of-turn constraints) checked.

use crate::compact::*;
use std::cell::Cell;

pub const GOD_MORTAL: u8 = 0;
pub const GOD_APOLLO: u8 = 1;
pub const GOD_ARTEMIS: u8 = 2;
pub const GOD_ATHENA: u8 = 3;
pub const GOD_ATLAS: u8 = 4;
pub const GOD_DEMETER: u8 = 5;
pub const GOD_HEPHAESTUS: u8 = 6;
pub const GOD_HERMES: u8 = 7;
pub const GOD_MINOTAUR: u8 = 8;
pub const GOD_PAN: u8 = 9;
pub const GOD_PROMETHEUS: u8 = 10;
pub const GOD_APHRODITE: u8 = 11;
pub const GOD_ARES: u8 = 12;
pub const GOD_BIA: u8 = 13;
pub const GOD_CHARON: u8 = 15;
pub const GOD_CHRONUS: u8 = 16;
pub const GOD_EROS: u8 = 19;
pub const GOD_HERA: u8 = 20;
pub const GOD_HESTIA: u8 = 21;
pub const GOD_HYPNUS: u8 = 22;
pub const GOD_LIMUS: u8 = 23;
pub const GOD_MEDUSA: u8 = 24;
pub const GOD_PERSEPHONE: u8 = 26;
pub const GOD_POSEIDON: u8 = 27;
pub const GOD_SELENE: u8 = 28;
pub const GOD_TRITON: u8 = 29;
pub const GOD_ZEUS: u8 = 30;
pub const GOD_HADES: u8 = 37;
pub const GOD_HARPIES: u8 = 38;
pub const GOD_TERPSICHORE: u8 = 44;
pub const GOD_URANIA: u8 = 45;
pub const GOD_ACHILLES: u8 = 46;
pub const GOD_ATALANTA: u8 = 48;
pub const GOD_BELLEROPHON: u8 = 49;
pub const GOD_HERACLES: u8 = 50;
pub const GOD_POLYPHEMUS: u8 = 54;
pub const GOD_THESEUS: u8 = 55;
pub const GOD_SCYLLA: u8 = 57;
pub const GOD_CASTOR: u8 = 58;
pub const GOD_MAENADS: u8 = 61;
pub const GOD_ASTERIA: u8 = 62;
pub const GOD_HIPPOLYTA: u8 = 63;
pub const GOD_IRIS: u8 = 65;
pub const GOD_PEGASUS: u8 = 67;

/// God IDs are `0..NUM_GODS` (sparse canonical IDs; see `is_supported_god`).
pub const NUM_GODS: usize = 68;
/// The gods self-play samples (`random_gods`): Mortal + the ten intro gods.
pub const INTRO_GODS: [u8; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
pub const SUPPORTED_GODS: [u8; 44] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 19, 20, 21, 22, 23, 24, 26, 27, 28, 29, 30, 37, 38, 44, 45,
    46, 48, 49, 50, 54, 55, 57, 58, 61, 62, 63, 65, 67,
];
pub const NUM_SUPPORTED_GODS: usize = SUPPORTED_GODS.len();
/// Dense index `0..NUM_SUPPORTED_GODS` for each canonical god ID (`u8::MAX` if unsupported).
pub const GOD_INDEX: [u8; NUM_GODS] = {
    let mut t = [u8::MAX; NUM_GODS];
    let mut i = 0;
    while i < NUM_SUPPORTED_GODS {
        t[SUPPORTED_GODS[i] as usize] = i as u8;
        i += 1;
    }
    t
};

#[inline(always)]
pub const fn index_of_god(g: u8) -> Option<usize> {
    if (g as usize) < NUM_GODS {
        let idx = GOD_INDEX[g as usize];
        if idx != u8::MAX {
            return Some(idx as usize);
        }
    }
    None
}

/// Dense `0..NUM_SUPPORTED_GODS` slot for an already-validated engine god ID.
#[inline(always)]
pub const fn god_slot(g: u8) -> usize {
    debug_assert!(is_supported_god(g));
    GOD_INDEX[g as usize] as usize
}
/// Display names by ID (`""` = not implemented).
pub const GOD_NAMES: [&str; NUM_GODS] = {
    let mut t = [""; NUM_GODS];
    t[0] = "Mortal";
    t[1] = "Apollo";
    t[2] = "Artemis";
    t[3] = "Athena";
    t[4] = "Atlas";
    t[5] = "Demeter";
    t[6] = "Hephaestus";
    t[7] = "Hermes";
    t[8] = "Minotaur";
    t[9] = "Pan";
    t[10] = "Prometheus";
    t[11] = "Aphrodite";
    t[12] = "Ares";
    t[13] = "Bia";
    t[15] = "Charon";
    t[16] = "Chronus";
    t[19] = "Eros";
    t[20] = "Hera";
    t[21] = "Hestia";
    t[22] = "Hypnus";
    t[23] = "Limus";
    t[24] = "Medusa";
    t[26] = "Persephone";
    t[27] = "Poseidon";
    t[28] = "Selene";
    t[29] = "Triton";
    t[30] = "Zeus";
    t[37] = "Hades";
    t[38] = "Harpies";
    t[44] = "Terpsichore";
    t[45] = "Urania";
    t[46] = "Achilles";
    t[48] = "Atalanta";
    t[49] = "Bellerophon";
    t[50] = "Heracles";
    t[54] = "Polyphemus";
    t[55] = "Theseus";
    t[57] = "Scylla";
    t[58] = "Castor & Pollux";
    t[61] = "Maenads";
    t[62] = "Asteria";
    t[63] = "Hippolyta";
    t[65] = "Iris";
    t[67] = "Pegasus";
    t
};

#[inline]
pub const fn is_supported_god(g: u8) -> bool {
    (g as usize) < NUM_GODS && !GOD_NAMES[g as usize].is_empty()
}

pub const DOME_TURN_OFFSET: i32 = TURN_ACTION_OFFSET + 128;
pub const MOVE_ONLY_OFFSET: i32 = DOME_TURN_OFFSET + 128;
pub const BUILD_ONLY_OFFSET: i32 = MOVE_ONLY_OFFSET + 16;
pub const PASS_ACTION: i32 = BUILD_ONLY_OFFSET + 16;
pub const SQUARE_OFFSET: i32 = PASS_ACTION + 1;
pub const DOME_ONLY_OFFSET: i32 = SQUARE_OFFSET + NUM_SQUARES as i32;
pub const ZEUS_OFFSET: i32 = DOME_ONLY_OFFSET + 16;
pub const FORCE_OFFSET: i32 = ZEUS_OFFSET + 16;
/// One past the last action ID (`compact::ACTION_SPACE`).
pub const ACTION_END: i32 = FORCE_OFFSET + 16;

/// Extra moves allowed per Hermes / Triton / Atalanta turn (engine simplification of "any
/// number" so search trees stay finite).
pub const CHAIN_MAX_STEPS: u8 = 4;
pub const HERMES_MAX_STEPS: u8 = CHAIN_MAX_STEPS;
/// `status_flags` layout: bit p = player p is Athena and moved up on their last turn;
/// bits 4..=6 = sub-turn step counter (Hermes, Triton, Atalanta, Poseidon, Polyphemus).
const STEP_SHIFT: u8 = 4;
const STEP_MASK: u8 = 0b111 << STEP_SHIFT;

// Unified sub-turn stages (`stage == STAGE_NONE` iff `sub_phase != PHASE_SUB_TURN`).
// Stages 1..=11 bind `active_worker` to a specific worker; stages 12..=16 leave `active_worker`
// at 0 so worker-order transpositions (e.g. Terpsichore/Castor moving A then B vs B then A)
// produce identical states and hashes.
pub const STAGE_NONE: u8 = 0;
pub const STAGE_BUILD: u8 = 1; // active worker must build (Triton / Charon / Scylla)
pub const STAGE_CHAIN: u8 = 2; // Triton / Atalanta: active worker moves again or builds
pub const STAGE_DEMETER: u8 = 3; // active worker: optional build != prev_square, or pass
pub const STAGE_HEPHAESTUS: u8 = 4; // active worker: optional non-dome build == prev_square, or pass
pub const STAGE_HESTIA: u8 = 5; // active worker: optional non-perimeter build, or pass
pub const STAGE_POSEIDON: u8 = 6; // active = unmoved ground worker: up to 2 builds, or pass
pub const STAGE_END: u8 = 7; // active = moved worker: end-of-turn square target, or pass
pub const STAGE_SCYLLA: u8 = 8; // active = moved worker: drag neighbour of prev_square, then build
pub const STAGE_ARTEMIS: u8 = 9; // active = moved worker: full turn (!= prev_square) or build
pub const STAGE_PROMETHEUS: u8 = 10; // active = pre-built worker: full turn without climbing
pub const STAGE_ACHILLES: u8 = 11; // active = pre-built worker: full turn (climbing allowed)
pub const STAGE_HERMES: u8 = 12; // unbound: further level steps or build with either worker
pub const STAGE_HERACLES: u8 = 13; // unbound: dome neighbours with either worker, or pass
pub const STAGE_TERP: u8 = 14; // unbound: move unmoved workers, then build with unbuilt workers
pub const STAGE_CASTOR_MOVE: u8 = 15; // unbound: move the remaining worker, no build
pub const STAGE_CASTOR_BUILD: u8 = 16; // unbound: build with the remaining worker
pub const NUM_STAGES: usize = 17;

#[inline(always)]
pub const fn stage_binds_worker(stage: u8) -> bool {
    matches!(
        stage,
        STAGE_BUILD
            | STAGE_CHAIN
            | STAGE_DEMETER
            | STAGE_HEPHAESTUS
            | STAGE_HESTIA
            | STAGE_POSEIDON
            | STAGE_END
            | STAGE_SCYLLA
            | STAGE_ARTEMIS
            | STAGE_PROMETHEUS
            | STAGE_ACHILLES
    )
}

// Per-worker `tags` (they travel with the worker through sorting and symmetries).
pub const TAG_FEMALE: u8 = 1;
pub const TAG_APHRODITE: u8 = 2; // neighboured Aphrodite at turn start: must at turn end
pub const TAG_FROZEN: u8 = 4; // Hypnus: cannot move this turn
pub const TAG_MOVED: u8 = 8; // Terpsichore / Castor bookkeeping
pub const TAG_BUILT: u8 = 16;
const TRANSIENT_TAGS: u8 = TAG_APHRODITE | TAG_FROZEN | TAG_MOVED | TAG_BUILT;

// `turn_flags` bit indices: p = p moved up during their latest turn, TF_DOWN + p = moved down,
// TF_PERSEPHONE + p = Persephone obliges p to move up this turn.
pub const TF_DOWN: usize = 2;
pub const TF_PERSEPHONE: usize = 4;

thread_local! {
    /// > 0 while exploring hypothetical continuations (legality filter, Persephone, win
    /// probes): `end_turn` then skips the opponent's start-of-turn bookkeeping and no-move
    /// check, which would otherwise recurse one ply deeper per call.
    static PROBING: Cell<u32> = const { Cell::new(0) };
    #[cfg(test)]
    static PROBE_APPLIES: Cell<u32> = const { Cell::new(0) };
}

fn probing<T>(f: impl FnOnce() -> T) -> T {
    PROBING.with(|p| p.set(p.get() + 1));
    let out = f();
    PROBING.with(|p| p.set(p.get() - 1));
    out
}

/// Urania's neighbours: the board wraps like a torus (D4-symmetric, so augmentation holds).
pub const WRAP_NEIGHBORS: [[u8; 8]; NUM_SQUARES] = {
    let mut t = [[0u8; 8]; NUM_SQUARES];
    let mut sq = 0;
    while sq < NUM_SQUARES {
        let (r, c) = ((sq / BOARD_SIZE) as i32, (sq % BOARD_SIZE) as i32);
        let mut d = 0;
        while d < 8 {
            let n = BOARD_SIZE as i32;
            let (nr, nc) = ((r + DIR_DELTAS[d].0 + n) % n, (c + DIR_DELTAS[d].1 + n) % n);
            t[sq][d] = (nr as usize * BOARD_SIZE + nc as usize) as u8;
            d += 1;
        }
        sq += 1;
    }
    t
};

#[inline(always)]
pub const fn is_perimeter(sq: usize) -> bool {
    let (r, c) = (sq / BOARD_SIZE, sq % BOARD_SIZE);
    r == 0 || c == 0 || r == BOARD_SIZE - 1 || c == BOARD_SIZE - 1
}

/// Edge membership bits (N, E, S, W) for Eros' setup rule.
const fn edges(sq: usize) -> u8 {
    let (r, c) = (sq / BOARD_SIZE, sq % BOARD_SIZE);
    (r == 0) as u8 | ((c == BOARD_SIZE - 1) as u8) << 1 | ((r == BOARD_SIZE - 1) as u8) << 2 | ((c == 0) as u8) << 3
}

#[inline(always)]
pub const fn encode_dome_turn(w: usize, m: usize, b: usize) -> i32 {
    DOME_TURN_OFFSET + (w * 64 + m * 8 + b) as i32
}
#[inline(always)]
pub const fn encode_move_only(w: usize, m: usize) -> i32 {
    MOVE_ONLY_OFFSET + (w * 8 + m) as i32
}
#[inline(always)]
pub const fn encode_build_only(w: usize, d: usize) -> i32 {
    BUILD_ONLY_OFFSET + (w * 8 + d) as i32
}
#[inline(always)]
pub const fn encode_square(sq: usize) -> i32 {
    SQUARE_OFFSET + sq as i32
}
#[inline(always)]
pub const fn encode_dome_only(w: usize, d: usize) -> i32 {
    DOME_ONLY_OFFSET + (w * 8 + d) as i32
}
#[inline(always)]
pub const fn encode_zeus(w: usize, m: usize) -> i32 {
    ZEUS_OFFSET + (w * 8 + m) as i32
}
#[inline(always)]
pub const fn encode_force(w: usize, d: usize) -> i32 {
    FORCE_OFFSET + (w * 8 + d) as i32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Place(usize),
    Turn { w: usize, m: usize, b: usize, dome: bool },
    MoveOnly { w: usize, m: usize },
    BuildOnly { w: usize, d: usize },
    Pass,
    Square(usize),
    DomeOnly { w: usize, d: usize },
    Zeus { w: usize, m: usize },
    Force { w: usize, d: usize },
}

#[inline]
pub fn decode_action(a: i32) -> ActionKind {
    let wd = |off: i32| ((a - off) as usize / 8, (a - off) as usize % 8);
    if a < TURN_ACTION_OFFSET {
        ActionKind::Place(a as usize)
    } else if a < DOME_TURN_OFFSET {
        let (w, m, b) = decode_turn_action(a);
        ActionKind::Turn { w, m, b, dome: false }
    } else if a < MOVE_ONLY_OFFSET {
        let (w, m, b) = decode_turn_action(a - 128);
        ActionKind::Turn { w, m, b, dome: true }
    } else if a < BUILD_ONLY_OFFSET {
        let (w, m) = wd(MOVE_ONLY_OFFSET);
        ActionKind::MoveOnly { w, m }
    } else if a < PASS_ACTION {
        let (w, d) = wd(BUILD_ONLY_OFFSET);
        ActionKind::BuildOnly { w, d }
    } else if a == PASS_ACTION {
        ActionKind::Pass
    } else if a < DOME_ONLY_OFFSET {
        ActionKind::Square((a - SQUARE_OFFSET) as usize)
    } else if a < ZEUS_OFFSET {
        let (w, d) = wd(DOME_ONLY_OFFSET);
        ActionKind::DomeOnly { w, d }
    } else if a < FORCE_OFFSET {
        let (w, m) = wd(ZEUS_OFFSET);
        ActionKind::Zeus { w, m }
    } else {
        let (w, d) = wd(FORCE_OFFSET);
        ActionKind::Force { w, d }
    }
}

/// Result of a (possibly god-assisted) worker move.
#[derive(Clone, Copy)]
pub(crate) struct Moved {
    pub(crate) workers: [[u8; 2]; 2],
    pub(crate) to: usize,
    pub(crate) win: bool,
    pub(crate) up: bool,
    pub(crate) down: bool,
    /// Bellerophon spent his once-per-game climb.
    pub(crate) used_power: bool,
}

impl CompactSantoriniState {
    pub fn new_with_gods(gods: [u8; 2]) -> Self {
        let mut s = Self::new();
        s.gods = gods;
        s.current_player = s.first_placer();
        s
    }

    /// Who places first: player 0, unless only player 1 is Bia ("place your workers first").
    #[inline]
    pub(crate) fn first_placer(&self) -> u8 {
        (self.gods[1] == GOD_BIA && self.gods[0] != GOD_BIA) as u8
    }

    /// `from_position` for a god matchup.
    pub fn from_position_with_gods(height: [u8; NUM_SQUARES], workers: [[u8; 2]; 2], current_player: u8, gods: [u8; 2]) -> Self {
        Self::from_position_ext(height, workers, current_player, gods, 0, 0, 0, false)
    }

    /// Extended mid-game constructor supporting custom Pantheon positions:
    /// `low_domes` bitmask, once-per-game `power_used` bits, `athena_up` climb-lock bits,
    /// `1..=2` workers per side (`UNPLACED` for assassinated/petrified workers), and
    /// `allow_l3` for live god positions where a worker legally stands on level 3 without
    /// having won (Zeus, Pegasus, Hera, Minotaur/Charon/Scylla/Harpies forced moves).
    #[allow(clippy::too_many_arguments)]
    pub fn from_position_ext(
        height: [u8; NUM_SQUARES],
        workers: [[u8; 2]; 2],
        current_player: u8,
        gods: [u8; 2],
        low_domes: u32,
        power_used: u8,
        athena_up: u8,
        allow_l3: bool,
    ) -> Self {
        let mut s = Self::new_with_gods(gods);
        s.height = height;
        s.workers = workers;
        s.low_domes = low_domes;
        s.power_used = power_used & 0b11;
        for p in 0..2 {
            if gods[p] == GOD_ATHENA && athena_up & (1 << p) != 0 {
                s.status_flags |= 1 << p;
            }
            // Convention: worker 0 as given is the female; passing `[UNPLACED, male_sq]`
            // represents a Selene/Hippolyta position whose female worker was removed.
            if s.workers[p][0] != UNPLACED && (gods[p] == GOD_SELENE || gods[p] == GOD_HIPPOLYTA) {
                s.tags[p][0] |= TAG_FEMALE;
            }
        }
        s.sort_workers();
        s.sub_phase = PHASE_TURN;
        s.current_player = current_player;
        s.turn_count = 4 + height.iter().map(|&h| h as u16).sum::<u16>();
        if !allow_l3 || s.is_mortal_game() {
            for p in 0..2 {
                if s.workers[p].iter().any(|&w| w != UNPLACED && height[w as usize] == 3) {
                    s.game_over = true;
                    s.winner = p as i8;
                }
            }
        }
        if !s.is_mortal_game() {
            s.begin_turn(current_player as usize);
        }
        if !s.game_over && !s.has_valid_action() {
            s.game_over = true;
            s.winner = 1 - current_player as i8;
        }
        s
    }

    #[inline(always)]
    pub fn is_mortal_game(&self) -> bool {
        self.gods == [GOD_MORTAL, GOD_MORTAL]
    }

    /// Is `p` barred from moving up by an opposing Athena?
    #[inline(always)]
    pub fn athena_restricts(&self, p: usize) -> bool {
        self.gods[1 - p] == GOD_ATHENA && self.status_flags & (1 << (1 - p)) != 0
    }

    #[inline(always)]
    pub fn steps(&self) -> u8 {
        (self.status_flags & STEP_MASK) >> STEP_SHIFT
    }

    #[inline(always)]
    fn set_steps(&mut self, n: u8) {
        self.status_flags = (self.status_flags & !STEP_MASK) | (n << STEP_SHIFT);
    }

    #[inline(always)]
    fn used(&self, p: usize) -> bool {
        self.power_used & (1 << p) != 0
    }

    /// Neighbour of `sq` in `d` for player `p`'s own moves/builds (Urania wraps).
    #[inline(always)]
    fn nb(&self, p: usize, sq: usize, d: usize) -> u8 {
        if self.gods[p] == GOD_URANIA {
            WRAP_NEIGHBORS[sq][d]
        } else {
            NEIGHBORS[sq][d]
        }
    }

    #[inline(always)]
    fn occupied_in(workers: &[[u8; 2]; 2], sq: u8) -> bool {
        workers.iter().flatten().any(|&x| x == sq)
    }

    fn female_index(&self, p: usize) -> Option<usize> {
        (0..2).find(|&i| self.workers[p][i] != UNPLACED && self.tags[p][i] & TAG_FEMALE != 0)
    }

    /// Can `p` build on `sq` (after the workers moved to `workers`)? `dome`: the build is a
    /// dome (by power, or a normal build on level 3), which Limus allows.
    #[inline]
    fn can_build_at(&self, p: usize, workers: &[[u8; 2]; 2], sq: u8, dome: bool) -> bool {
        if sq == OFF_BOARD || self.height[sq as usize] >= DOME || Self::occupied_in(workers, sq) {
            return false;
        }
        if self.gods[1 - p] == GOD_LIMUS && !dome && self.height[sq as usize] < 3 {
            return !workers[1 - p].iter().any(|&l| l != UNPLACED && NEIGHBORS[l as usize].contains(&sq));
        }
        true
    }

    /// Moves `p`'s worker `w` in direction `m`, applying every god rule that shapes a move
    /// (own: Apollo, Minotaur, Pegasus, Bellerophon, Iris, Urania, Bia, Pan; opposing: Athena,
    /// Hades, Hera, Hypnus, Hippolyta, Harpies).
    pub(crate) fn try_move(&self, p: usize, w: usize, m: usize) -> Option<Moved> {
        let from = self.workers[p][w];
        if from == UNPLACED {
            return None;
        }
        let from = from as usize;
        let (god, og) = (self.gods[p], self.gods[1 - p]);
        if self.tags[p][w] & TAG_FROZEN != 0 {
            return None;
        }
        let hippolyta = god == GOD_HIPPOLYTA || og == GOD_HIPPOLYTA;
        if hippolyta && m.is_multiple_of(2) && !(god == GOD_HIPPOLYTA && self.tags[p][w] & TAG_FEMALE != 0) {
            return None;
        }
        let mut to = self.nb(p, from, m);
        if to == OFF_BOARD {
            return None;
        }
        let mut workers = self.workers;
        let hf = self.height[from];
        let restricted = self.athena_restricts(p);
        if god == GOD_IRIS && Self::occupied_in(&self.workers, to) {
            // Iris: over any neighbouring worker, onto the free space behind, at any level.
            to = NEIGHBORS[to as usize][m];
            if to == OFF_BOARD || self.height[to as usize] >= DOME || Self::occupied_in(&self.workers, to) {
                return None;
            }
            if restricted && self.height[to as usize] > hf {
                return None;
            }
        } else {
            let ht = self.height[to as usize];
            let max_up = if restricted {
                0
            } else if god == GOD_PEGASUS {
                3
            } else if god == GOD_BELLEROPHON && !self.used(p) {
                2
            } else {
                1
            };
            if ht >= DOME || ht > hf + max_up || self.workers[p][1 - w] == to {
                return None;
            }
            if let Some(k) = self.workers[1 - p].iter().position(|&o| o == to) {
                match god {
                    // 2E Apollo: only onto the same or a lower level.
                    GOD_APOLLO if ht <= hf => workers[1 - p][k] = from as u8,
                    GOD_MINOTAUR => {
                        let push = NEIGHBORS[to as usize][m];
                        if push == OFF_BOARD || self.height[push as usize] >= DOME || Self::occupied_in(&self.workers, push) {
                            return None;
                        }
                        workers[1 - p][k] = push;
                    }
                    _ => return None,
                }
            }
        }
        let to = to as usize;
        let ht = self.height[to];
        if og == GOD_HADES && ht < hf {
            return None;
        }
        workers[p][w] = to as u8;
        let mut win = (ht == 3 && ht > hf && !(god == GOD_PEGASUS && ht > hf + 1)) || (god == GOD_PAN && hf >= ht + 2);
        if og == GOD_HERA && is_perimeter(to) {
            win = false;
        }
        let used_power = god == GOD_BELLEROPHON && ht == hf + 2;
        if god == GOD_BIA {
            let next = NEIGHBORS[to][m];
            if let Some(k) = workers[1 - p].iter().position(|&o| o != UNPLACED && o == next) {
                workers[1 - p][k] = UNPLACED;
            }
        }
        let mut fin = to;
        if og == GOD_HARPIES && !win {
            loop {
                let n = NEIGHBORS[fin][m];
                if n == OFF_BOARD
                    || self.height[n as usize] >= DOME
                    || self.height[n as usize] > self.height[fin]
                    || Self::occupied_in(&workers, n)
                {
                    break;
                }
                fin = n as usize;
            }
            workers[p][w] = fin as u8;
        }
        Some(Moved { workers, to: fin, win, up: ht > hf, down: ht < hf, used_power })
    }

    /// Generic (god-aware) action generator, ascending action order. Every god game filters
    /// raw candidates so that every emitted action can complete the turn legally.
    ///
    /// Why the `!has_end_constraint` and Persephone single-ply fast paths:
    /// - When neither Aphrodite nor an unsatisfied Persephone obligation constrains the end of
    ///   the turn, `end_constraints_ok` is unconditionally true, and actions that end the turn
    ///   directly (`Turn`, `Pass`, `Zeus`, `STAGE_END` `Square`) or stay in an optional stage
    ///   offering `PASS_ACTION` complete legally without cloning and probing the state.
    /// - Under Persephone alone (`og != GOD_APHRODITE`), once `turn_flags & (1 << p) != 0` the
    ///   climb obligation is already satisfied for the rest of the turn; before then, single-move
    ///   `Turn` / `Zeus` actions never enter a stage that moves again, so their legality reduces
    ///   in `O(1)` to `move_step_climbs(p, w, m)`, while `STAGE_CHAIN` (`Atalanta` / `Triton`)
    ///   shares a DAG memo table across `MoveOnly` candidates.
    pub(crate) fn for_each_action_gods(&self, f: &mut dyn FnMut(i32) -> bool) {
        let p = self.current_player as usize;
        let (god, og) = (self.gods[p], self.gods[1 - p]);
        let must_climb = self.turn_flags & (1 << (TF_PERSEPHONE + p)) != 0
            && self.turn_flags & (1 << p) == 0;
        let has_end_constraint = og == GOD_APHRODITE || must_climb;
        let mut chain_memo = [([[0u32; 2]; (CHAIN_MAX_STEPS + 1) as usize], [[0u32; 2]; (CHAIN_MAX_STEPS + 1) as usize]); 2];
        self.gen_raw(&mut |a| {
            if !has_end_constraint && a < MOVE_ONLY_OFFSET {
                return f(a);
            }
            if og != GOD_APHRODITE && must_climb {
                match decode_action(a) {
                    ActionKind::Turn { w, m, .. } | ActionKind::Zeus { w, m } => {
                        return if self.move_step_climbs(p, w, m) { f(a) } else { true };
                    }
                    ActionKind::Force { .. } => return true,
                    _ => {}
                }
            }
            let needs_check = has_end_constraint
                || match decode_action(a) {
                    ActionKind::MoveOnly { w, m } => {
                        og == GOD_LIMUS
                            || match (self.stage, god) {
                                // Vacated square (or penultimate Harpies square) is guaranteed
                                // adjacent, unoccupied, and < DOME, so a follow-up BuildOnly exists;
                                // Heracles always has PASS_ACTION.
                                (_, GOD_ARTEMIS | GOD_HERMES | GOD_TRITON | GOD_HERACLES) => false,
                                (STAGE_CHAIN, GOD_ATALANTA) => false,
                                (STAGE_NONE, GOD_ATALANTA) => {
                                    // Step 2 can always step back onto `from` (and build `to`)
                                    // unless stepping `from -> to` dropped >1 level or Athena /
                                    // Hades / Harpies alters the return step.
                                    let from = self.workers[p][w] as usize;
                                    let to = NEIGHBORS[from][m] as usize;
                                    og == GOD_HADES
                                        || og == GOD_HARPIES
                                        || self.athena_restricts(p)
                                        || self.height[from] > self.height[to] + 1
                                }
                                _ => true,
                            }
                    }
                    ActionKind::BuildOnly { .. } => matches!(
                        (self.stage, god),
                        (STAGE_NONE, GOD_PROMETHEUS | GOD_ACHILLES | GOD_CASTOR) | (STAGE_TERP, _)
                    ),
                    ActionKind::Force { .. } => true,
                    ActionKind::Square(_) => self.stage == STAGE_SCYLLA,
                    _ => false,
                };
            if needs_check {
                let slot = if self.stage == STAGE_CHAIN {
                    self.active_worker as usize
                } else if let ActionKind::MoveOnly { w, .. } = decode_action(a) {
                    w
                } else {
                    0
                };
                if !self.completes_legally_with_memo(a, &mut chain_memo[slot]) {
                    return true;
                }
            }
            f(a)
        });
    }

    /// Does worker `w`'s step in direction `m` climb (`mv.up` in `try_move`)?
    #[inline]
    fn move_step_climbs(&self, p: usize, w: usize, m: usize) -> bool {
        let from = self.workers[p][w];
        if from == UNPLACED {
            return false;
        }
        let from = from as usize;
        let mut to = self.nb(p, from, m);
        if to == OFF_BOARD {
            return false;
        }
        if self.gods[p] == GOD_IRIS && Self::occupied_in(&self.workers, to) {
            to = NEIGHBORS[to as usize][m];
            if to == OFF_BOARD {
                return false;
            }
        }
        self.height[to as usize] > self.height[from]
    }

    /// Can raw candidate `a` be part of a legal turn? It either leads to a sub-turn with a
    /// legal continuation, or ends the turn satisfying Aphrodite / Persephone.
    fn completes_legally(&self, a: i32) -> bool {
        let mut memo = ([[0u32; 2]; (CHAIN_MAX_STEPS + 1) as usize], [[0u32; 2]; (CHAIN_MAX_STEPS + 1) as usize]);
        self.completes_legally_with_memo(a, &mut memo)
    }

    fn completes_legally_with_memo(
        &self,
        a: i32,
        chain_memo: &mut ([[u32; 2]; (CHAIN_MAX_STEPS + 1) as usize], [[u32; 2]; (CHAIN_MAX_STEPS + 1) as usize]),
    ) -> bool {
        let p = self.current_player as usize;
        probing(|| {
            let mut c = *self;
            c.apply_action_gods(a);
            if !c.game_over && c.current_player as usize == p {
                // With no move step left (and no Scylla drag, which moves an opponent), the end
                // constraints can only stay or worsen: check them now. Likewise, neither
                // `STAGE_HERMES` nor `STAGE_PROMETHEUS` ever allows moving up, so they can never
                // satisfy an unsatisfied Persephone obligation.
                if c.stage != STAGE_SCYLLA && !c.can_still_move() && !self.end_constraints_ok(&c) {
                    return false;
                }
                if matches!(c.stage, STAGE_HERMES | STAGE_PROMETHEUS)
                    && self.turn_flags & (1 << (TF_PERSEPHONE + p)) != 0
                    && c.turn_flags & (1 << p) == 0
                {
                    return false;
                }
                if c.stage == STAGE_CHAIN {
                    return c.completes_chain_legally(chain_memo);
                }
                return c.has_valid_action();
            }
            self.end_constraints_ok(&c)
        })
    }

    /// Exact DAG memoization over `(steps, moved_up, active_sq)` in `STAGE_CHAIN` (`Atalanta` /
    /// `Triton`), where `steps` strictly increases (`0..=4`) and all other board/worker fields
    /// are invariant for a given starting worker.
    fn completes_chain_legally(
        &self,
        memo: &mut ([[u32; 2]; (CHAIN_MAX_STEPS + 1) as usize], [[u32; 2]; (CHAIN_MAX_STEPS + 1) as usize]),
    ) -> bool {
        let p = self.current_player as usize;
        let (god, og) = (self.gods[p], self.gods[1 - p]);
        let steps = self.steps() as usize;
        let up_bit = ((self.turn_flags >> p) & 1) as usize;
        let persephone = self.turn_flags & (1 << (TF_PERSEPHONE + p)) != 0;
        if og != GOD_APHRODITE
            && og != GOD_LIMUS
            && (!persephone || up_bit == 1)
            && (god != GOD_ATALANTA || steps >= 1)
        {
            return true;
        }
        if persephone && up_bit == 0 && steps == CHAIN_MAX_STEPS as usize {
            return false;
        }
        let sq = self.workers[p][self.active_worker as usize] as usize;
        let mask = 1u32 << sq;
        if memo.0[steps][up_bit] & mask != 0 {
            return memo.1[steps][up_bit] & mask != 0;
        }
        memo.0[steps][up_bit] |= mask;
        let mut ok = false;
        self.gen_raw(&mut |a| {
            if persephone && up_bit == 0 && a >= BUILD_ONLY_OFFSET {
                return true;
            }
            let mut c = *self;
            c.apply_action_gods(a);
            ok = if !c.game_over && c.current_player as usize == p {
                if c.stage == STAGE_CHAIN {
                    c.completes_chain_legally(memo)
                } else {
                    (c.stage == STAGE_SCYLLA || c.can_still_move() || self.end_constraints_ok(&c))
                        && c.has_valid_action()
                }
            } else {
                !(c.game_over && c.winner != p as i8) && self.end_constraints_ok(&c)
            };
            !ok
        });
        if ok {
            memo.1[steps][up_bit] |= mask;
        }
        ok
    }

    /// Aphrodite / Persephone constraints on the side to move's turn ending in `c`.
    fn end_constraints_ok(&self, c: &Self) -> bool {
        let p = self.current_player as usize;
        if self.gods[1 - p] == GOD_APHRODITE {
            for i in 0..2 {
                let w = c.workers[p][i];
                if w != UNPLACED
                    && c.tags[p][i] & TAG_APHRODITE != 0
                    && !c.workers[1 - p].iter().any(|&o| o != UNPLACED && NEIGHBORS[w as usize].contains(&o))
                {
                    return false;
                }
            }
        }
        !(self.turn_flags & (1 << (TF_PERSEPHONE + p)) != 0 && c.turn_flags & (1 << p) == 0)
    }

    fn gen_raw(&self, f: &mut dyn FnMut(i32) -> bool) {
        let p = self.current_player as usize;
        let god = self.gods[p];
        let stage = self.stage;
        let active = self.active_worker as usize;
        let steps = self.steps();
        let used = self.used(p);
        let me = self.workers[p];
        let placed = |w: usize| me[w] != UNPLACED;

        // --- Full move+build turns (Base range, then dome-turns: Atlas / Selene).
        let turn_workers: &[usize] = match (stage, god) {
            (STAGE_NONE, GOD_TERPSICHORE) => &[],
            (STAGE_NONE, _) => &[0, 1],
            (STAGE_ARTEMIS | STAGE_PROMETHEUS | STAGE_ACHILLES, _) => std::slice::from_ref(&active),
            _ => &[],
        };
        for dome in [false, true] {
            if dome && (stage != STAGE_NONE || (god != GOD_ATLAS && god != GOD_SELENE)) {
                continue;
            }
            for &w in turn_workers {
                for m in 0..8 {
                    let Some(mv) = self.try_move(p, w, m) else { continue };
                    if stage == STAGE_ARTEMIS && mv.to as u8 == self.prev_square {
                        continue; // Artemis may not return to her start square
                    }
                    if stage == STAGE_PROMETHEUS && mv.up {
                        continue; // Prometheus built first: no moving up
                    }
                    if god == GOD_TRITON && is_perimeter(mv.to) {
                        continue; // Triton's perimeter moves are move-only steps (he may go on)
                    }
                    if mv.win {
                        if !dome && !f(encode_turn_action(w, m, opposite_dir(m))) {
                            return;
                        }
                        continue;
                    }
                    let base = if dome && god == GOD_SELENE {
                        match self.female_index(p) {
                            Some(i) => mv.workers[p][i] as usize,
                            None => continue,
                        }
                    } else {
                        mv.to
                    };
                    for b in 0..8 {
                        let t = self.nb(p, base, b);
                        let ok = if !dome {
                            t != OFF_BOARD && self.can_build_at(p, &mv.workers, t, self.height[t as usize] == 3)
                        } else if god == GOD_ATLAS {
                            self.can_build_at(p, &mv.workers, t, true) && self.height[t as usize] < 3
                        } else {
                            // Selene: skip L3 squares a normal build could dome anyway.
                            self.can_build_at(p, &mv.workers, t, true)
                                && !(self.height[t as usize] == 3 && (0..8).any(|k| self.nb(p, mv.to, k) == t))
                        };
                        if !ok {
                            continue;
                        }
                        let a = if dome { encode_dome_turn(w, m, b) } else { encode_turn_action(w, m, b) };
                        if !f(a) {
                            return;
                        }
                    }
                }
            }
        }

        // --- Move-only steps.
        let terp_moving = god == GOD_TERPSICHORE && (0..2).any(|i| placed(i) && self.tags[p][i] & TAG_MOVED == 0);
        for w in 0..2 {
            if !placed(w) {
                continue;
            }
            for m in 0..8 {
                let ok = match (stage, god) {
                    (STAGE_NONE, GOD_ARTEMIS) => self.try_move(p, w, m).is_some_and(|mv| !mv.win),
                    (STAGE_NONE | STAGE_HERMES, GOD_HERMES) if steps < CHAIN_MAX_STEPS => {
                        let from = me[w] as usize;
                        let to = NEIGHBORS[from][m];
                        to != OFF_BOARD && self.height[to as usize] == self.height[from] && self.try_move(p, w, m).is_some()
                    }
                    (STAGE_NONE, GOD_TRITON) => self.try_move(p, w, m).is_some_and(|mv| is_perimeter(mv.to)),
                    (STAGE_CHAIN, _) if w == active && steps < CHAIN_MAX_STEPS => self.try_move(p, w, m).is_some(),
                    (STAGE_NONE, GOD_ATALANTA) if !used => self.try_move(p, w, m).is_some_and(|mv| !mv.win),
                    (STAGE_NONE, GOD_SCYLLA) => self.try_move(p, w, m).is_some_and(|mv| {
                        !mv.win && mv.workers[1 - p].iter().any(|&o| o != UNPLACED && NEIGHBORS[me[w] as usize].contains(&o))
                    }),
                    (STAGE_NONE, GOD_HERACLES) if !used => self.try_move(p, w, m).is_some_and(|mv| !mv.win),
                    (STAGE_NONE, GOD_TERPSICHORE) | (STAGE_TERP, _) if terp_moving => {
                        self.tags[p][w] & TAG_MOVED == 0 && self.try_move(p, w, m).is_some()
                    }
                    (STAGE_NONE, GOD_CASTOR) => self.try_move(p, w, m).is_some_and(|mv| !mv.win),
                    (STAGE_CASTOR_MOVE, _) => self.tags[p][w] & TAG_MOVED == 0 && self.try_move(p, w, m).is_some(),
                    _ => false,
                };
                if ok && !f(encode_move_only(w, m)) {
                    return;
                }
            }
        }

        // --- Build-only steps.
        for w in 0..2 {
            if !placed(w) {
                continue;
            }
            let worker_ok = match (stage, god) {
                (STAGE_NONE, GOD_HERMES | GOD_PROMETHEUS | GOD_CASTOR) | (STAGE_HERMES, _) => true,
                (STAGE_NONE, GOD_ACHILLES) => !used,
                (STAGE_BUILD | STAGE_ARTEMIS | STAGE_DEMETER | STAGE_HEPHAESTUS | STAGE_HESTIA, _) => w == active,
                (STAGE_CHAIN, _) => w == active && (god != GOD_ATALANTA || steps >= 1),
                (STAGE_POSEIDON, _) => w == active && steps < 2,
                (STAGE_CASTOR_BUILD, _) => self.tags[p][w] & TAG_BUILT == 0,
                (STAGE_TERP, _) => !terp_moving && self.tags[p][w] & TAG_BUILT == 0,
                _ => false,
            };
            if !worker_ok {
                continue;
            }
            let at = me[w] as usize;
            for d in 0..8 {
                let t = self.nb(p, at, d);
                if t == OFF_BOARD || !self.can_build_at(p, &self.workers, t, self.height[t as usize] == 3) {
                    continue;
                }
                let ok = match stage {
                    STAGE_DEMETER => t != self.prev_square,
                    STAGE_HEPHAESTUS => t == self.prev_square && self.height[t as usize] < 3,
                    STAGE_HESTIA => !is_perimeter(t as usize),
                    _ => true,
                };
                if ok && !f(encode_build_only(w, d)) {
                    return;
                }
            }
        }

        // --- Pass (decline an optional step).
        let pass = matches!(
            stage,
            STAGE_DEMETER | STAGE_HEPHAESTUS | STAGE_HESTIA | STAGE_POSEIDON | STAGE_END | STAGE_HERACLES
        );
        if pass && !f(PASS_ACTION) {
            return;
        }

        // --- Square targets.
        if stage == STAGE_END || stage == STAGE_SCYLLA {
            for t in 0..NUM_SQUARES {
                if self.square_target_ok(p, t) && !f(encode_square(t)) {
                    return;
                }
            }
        }

        // --- Heracles: domes on any neighbouring space, any worker.
        if stage == STAGE_HERACLES {
            for w in 0..2 {
                if !placed(w) {
                    continue;
                }
                for d in 0..8 {
                    if self.can_build_at(p, &self.workers, NEIGHBORS[me[w] as usize][d], true) && !f(encode_dome_only(w, d)) {
                        return;
                    }
                }
            }
        }

        // --- Zeus: move, then a block under the moved worker.
        if stage == STAGE_NONE && god == GOD_ZEUS {
            for w in 0..2 {
                for m in 0..8 {
                    let ok = self.try_move(p, w, m).is_some_and(|mv| {
                        !mv.win
                            && self.height[mv.to] < 3
                            && !(self.gods[1 - p] == GOD_LIMUS
                                && mv.workers[1 - p].iter().any(|&l| l != UNPLACED && NEIGHBORS[l as usize].contains(&(mv.to as u8))))
                    });
                    if ok && !f(encode_zeus(w, m)) {
                        return;
                    }
                }
            }
        }

        // --- Charon: force a neighbouring opponent behind your worker (instead of moving).
        if stage == STAGE_NONE && god == GOD_CHARON {
            for w in 0..2 {
                if !placed(w) {
                    continue;
                }
                let at = me[w] as usize;
                for d in 0..8 {
                    let (o, behind) = (NEIGHBORS[at][d], NEIGHBORS[at][opposite_dir(d)]);
                    let ok = o != OFF_BOARD
                        && self.workers[1 - p].contains(&o)
                        && behind != OFF_BOARD
                        && self.height[behind as usize] < DOME
                        && !Self::occupied_in(&self.workers, behind);
                    if ok && !f(encode_force(w, d)) {
                        return;
                    }
                }
            }
        }
    }

    /// Square targets for `STAGE_END` (per god) and `STAGE_SCYLLA`.
    fn square_target_ok(&self, p: usize, t: usize) -> bool {
        let t8 = t as u8;
        let h = self.height[t];
        let free = h < DOME && !Self::occupied_in(&self.workers, t8);
        let opp_here = self.workers[1 - p].contains(&t8);
        let mine = self.workers[p];
        let near = |pred: &dyn Fn(u8) -> bool| {
            mine.iter().any(|&w| w != UNPLACED && NEIGHBORS[w as usize].contains(&t8) && pred(self.height[w as usize]))
        };
        if self.stage == STAGE_SCYLLA {
            return opp_here && self.prev_square != UNPLACED && NEIGHBORS[self.prev_square as usize].contains(&t8);
        }
        match self.gods[p] {
            GOD_ARES => {
                let u = mine[1 - self.active_worker as usize];
                free && (1..=3).contains(&h) && u != UNPLACED && NEIGHBORS[u as usize].contains(&t8)
            }
            GOD_ASTERIA | GOD_POLYPHEMUS => free,
            GOD_MEDUSA => opp_here && near(&|hw| hw > h),
            GOD_THESEUS => opp_here && near(&|hw| h >= hw + 2),
            _ => false,
        }
    }

    /// God-aware `apply_action` (legal action assumed).
    pub(crate) fn apply_action_gods(&mut self, action: i32) {
        debug_assert_eq!(self.sub_phase == PHASE_SUB_TURN, self.stage != STAGE_NONE);
        #[cfg(test)]
        if PROBING.with(|c| c.get()) > 0 {
            PROBE_APPLIES.with(|c| c.set(c.get() + 1));
        }
        let p = self.current_player as usize;
        let god = self.gods[p];
        let stage = self.stage;
        match decode_action(action) {
            ActionKind::Place(_) => unreachable!("placement is handled by apply_action"),
            ActionKind::Turn { w, m, b, dome } => {
                let mv = self.try_move(p, w, m).expect("illegal god move");
                let moved_to = mv.to as u8;
                self.take_move(p, &mv);
                if mv.win {
                    return self.finish_win(p);
                }
                let t = if dome && god == GOD_SELENE {
                    let f = self.female_index(p).expect("Selene without a female worker");
                    self.nb(p, self.workers[p][f] as usize, b) as usize
                } else {
                    self.nb(p, mv.to, b) as usize
                };
                if dome {
                    self.dome(t);
                } else {
                    self.height[t] += 1;
                }
                if god == GOD_ATHENA {
                    self.status_flags = (self.status_flags & !(1 << p)) | ((mv.up as u8) << p);
                }
                if stage == STAGE_NONE {
                    let entered = match god {
                        GOD_DEMETER => self.enter_stage(STAGE_DEMETER, moved_to, t as u8),
                        GOD_HEPHAESTUS => self.enter_stage(STAGE_HEPHAESTUS, moved_to, t as u8),
                        GOD_HESTIA => self.enter_stage(STAGE_HESTIA, moved_to, UNPLACED),
                        _ => false,
                    };
                    if entered {
                        return;
                    }
                }
                self.finish_turn(p, moved_to);
            }
            ActionKind::MoveOnly { w, m } => {
                let from = self.workers[p][w];
                let mv = self.try_move(p, w, m).expect("illegal move-only step");
                self.take_move(p, &mv);
                if mv.win {
                    return self.finish_win(p);
                }
                let to = mv.to as u8;
                match (stage, god) {
                    (STAGE_NONE | STAGE_HERMES, GOD_HERMES) => {
                        self.set_steps(self.steps() + 1);
                        self.set_unbound_stage(STAGE_HERMES);
                    }
                    (STAGE_NONE, GOD_ARTEMIS) => self.set_stage(STAGE_ARTEMIS, to, from),
                    (_, GOD_TRITON) => {
                        let n = if stage == STAGE_CHAIN { self.steps() + 1 } else { 0 };
                        let chain = is_perimeter(mv.to) && n < CHAIN_MAX_STEPS;
                        self.set_stage(if chain { STAGE_CHAIN } else { STAGE_BUILD }, to, UNPLACED);
                        self.set_steps(n);
                    }
                    (_, GOD_ATALANTA) => {
                        let n = if stage == STAGE_CHAIN { self.steps() + 1 } else { 0 };
                        self.power_used |= 1 << p;
                        self.set_stage(STAGE_CHAIN, to, UNPLACED);
                        self.set_steps(n);
                    }
                    (STAGE_NONE, GOD_SCYLLA) => self.set_stage(STAGE_SCYLLA, to, from),
                    (STAGE_NONE, GOD_HERACLES) => {
                        self.power_used |= 1 << p;
                        self.set_unbound_stage(STAGE_HERACLES);
                    }
                    (_, GOD_TERPSICHORE) => {
                        self.tag(p, to, TAG_MOVED);
                        self.set_unbound_stage(STAGE_TERP);
                    }
                    (_, GOD_CASTOR) => {
                        self.tag(p, to, TAG_MOVED);
                        let other_left = (0..2).any(|i| self.workers[p][i] != UNPLACED && self.tags[p][i] & TAG_MOVED == 0);
                        if stage == STAGE_CASTOR_MOVE || !other_left {
                            return self.end_turn(p);
                        }
                        self.set_unbound_stage(STAGE_CASTOR_MOVE);
                    }
                    _ => unreachable!("move-only step for god {god} in stage {stage}"),
                }
            }
            ActionKind::BuildOnly { w, d } => {
                let at = self.workers[p][w];
                let t = self.nb(p, at as usize, d) as usize;
                self.height[t] += 1;
                match (stage, god) {
                    (STAGE_NONE, GOD_PROMETHEUS) => self.set_stage(STAGE_PROMETHEUS, at, UNPLACED),
                    (STAGE_NONE, GOD_ACHILLES) => {
                        self.power_used |= 1 << p;
                        self.set_stage(STAGE_ACHILLES, at, UNPLACED);
                    }
                    (STAGE_NONE, GOD_CASTOR) => {
                        self.tag(p, at, TAG_BUILT);
                        if self.workers[p][1 - w] == UNPLACED {
                            return self.end_turn(p);
                        }
                        self.set_unbound_stage(STAGE_CASTOR_BUILD);
                    }
                    (STAGE_BUILD | STAGE_CHAIN | STAGE_ARTEMIS, _) => self.finish_turn(p, at),
                    (STAGE_POSEIDON, _) => {
                        self.set_steps(self.steps() + 1);
                        if self.steps() >= 2 || !self.has_valid_action_except_pass() {
                            self.end_turn(p);
                        }
                    }
                    (STAGE_TERP, _) => {
                        self.tag(p, at, TAG_BUILT);
                        if (0..2).all(|i| self.workers[p][i] == UNPLACED || self.tags[p][i] & TAG_BUILT != 0) {
                            self.end_turn(p);
                        }
                    }
                    // STAGE_NONE (Hermes 0-step build), STAGE_HERMES, STAGE_DEMETER,
                    // STAGE_HEPHAESTUS, STAGE_HESTIA, STAGE_CASTOR_BUILD.
                    _ => self.end_turn(p),
                }
            }
            ActionKind::Pass => self.end_turn(p),
            ActionKind::Square(t) => {
                if stage == STAGE_SCYLLA {
                    let k = self.workers[1 - p].iter().position(|&o| o as usize == t).expect("Scylla target");
                    self.workers[1 - p][k] = self.prev_square;
                    self.sort_workers();
                    let active_sq = self.workers[p][self.active_worker as usize];
                    return self.set_stage(STAGE_BUILD, active_sq, UNPLACED);
                }
                match god {
                    GOD_ARES => self.height[t] -= 1,
                    GOD_ASTERIA => self.dome(t),
                    GOD_POLYPHEMUS => {
                        self.power_used |= 1 << p;
                        self.dome(t);
                        self.set_steps(self.steps() + 1);
                        if self.steps() < 2 && self.has_valid_action_except_pass() {
                            return;
                        }
                    }
                    GOD_MEDUSA => {
                        self.remove_worker(1 - p, t as u8);
                        self.dome(t);
                    }
                    GOD_THESEUS => {
                        self.power_used |= 1 << p;
                        self.remove_worker(1 - p, t as u8);
                        if self.has_valid_action_except_pass() {
                            return;
                        }
                    }
                    _ => unreachable!("square target for god {god}"),
                }
                self.end_turn(p);
            }
            ActionKind::DomeOnly { w, d } => {
                let t = NEIGHBORS[self.workers[p][w] as usize][d] as usize;
                self.dome(t);
            }
            ActionKind::Zeus { w, m } => {
                let mv = self.try_move(p, w, m).expect("illegal Zeus move");
                self.take_move(p, &mv);
                self.height[mv.to] += 1;
                self.finish_turn(p, mv.to as u8);
            }
            ActionKind::Force { w, d } => {
                let at = self.workers[p][w] as usize;
                let (o, behind) = (NEIGHBORS[at][d], NEIGHBORS[at][opposite_dir(d)]);
                let k = self.workers[1 - p].iter().position(|&x| x == o).expect("Charon target");
                self.workers[1 - p][k] = behind;
                self.sort_workers();
                self.set_stage(STAGE_BUILD, at as u8, UNPLACED);
            }
        }
    }

    /// Commits a move: workers, Bellerophon's spent power, up/down bookkeeping.
    fn take_move(&mut self, p: usize, mv: &Moved) {
        self.workers = mv.workers;
        if mv.used_power {
            self.power_used |= 1 << p;
        }
        self.turn_flags |= ((mv.up as u8) << p) | ((mv.down as u8) << (TF_DOWN + p));
        self.sort_workers();
    }

    fn dome(&mut self, t: usize) {
        if self.height[t] < 3 {
            self.low_domes |= 1 << t;
        }
        self.height[t] = DOME;
    }

    fn remove_worker(&mut self, p: usize, sq: u8) {
        let k = self.workers[p].iter().position(|&x| x == sq).expect("no worker to remove");
        self.workers[p][k] = UNPLACED;
        self.tags[p][k] = 0;
        self.sort_workers();
    }

    fn tag(&mut self, p: usize, sq: u8, tag: u8) {
        if let Some(k) = self.workers[p].iter().position(|&x| x == sq) {
            self.tags[p][k] |= tag;
        }
    }

    /// Enters a worker-bound sub-turn stage anchored on the worker standing at `active_sq`.
    fn set_stage(&mut self, stage: u8, active_sq: u8, prev: u8) {
        debug_assert!(stage_binds_worker(stage));
        let p = self.current_player as usize;
        self.sort_workers();
        self.sub_phase = PHASE_SUB_TURN;
        self.stage = stage;
        self.active_worker = self.workers[p].iter().position(|&x| x == active_sq).unwrap_or(0) as u8;
        self.prev_square = prev;
    }

    /// Enters a sub-turn stage not bound to a specific worker (`active_worker` stays 0).
    fn set_unbound_stage(&mut self, stage: u8) {
        debug_assert!(stage != STAGE_NONE && !stage_binds_worker(stage));
        self.sort_workers();
        self.sub_phase = PHASE_SUB_TURN;
        self.stage = stage;
        self.active_worker = 0;
        self.prev_square = UNPLACED;
    }

    /// Enters an optional stage if it offers something besides passing; returns whether it did.
    fn enter_stage(&mut self, stage: u8, active_sq: u8, prev: u8) -> bool {
        let saved = *self;
        self.set_stage(stage, active_sq, prev);
        if self.has_valid_action_except_pass() {
            return true;
        }
        *self = saved;
        false
    }

    fn has_valid_action_except_pass(&self) -> bool {
        let mut extra = false;
        self.for_each_action_gods(&mut |a| {
            extra = a != PASS_ACTION;
            !extra
        });
        extra
    }

    /// End of a god turn's main step: offer end-of-turn powers if any, else pass the turn.
    fn finish_turn(&mut self, p: usize, moved_sq: u8) {
        self.sort_workers();
        let used = self.used(p);
        let entered = match self.gods[p] {
            GOD_POSEIDON => {
                let unmoved = self.workers[p].iter().copied().find(|&x| x != UNPLACED && x != moved_sq);
                match unmoved {
                    Some(u) if self.height[u as usize] == 0 => {
                        self.set_steps(0);
                        self.enter_stage(STAGE_POSEIDON, u, UNPLACED)
                    }
                    _ => false,
                }
            }
            GOD_ASTERIA => self.turn_flags & (1 << (TF_DOWN + p)) != 0 && self.enter_stage(STAGE_END, moved_sq, UNPLACED),
            GOD_ARES | GOD_MEDUSA => self.enter_stage(STAGE_END, moved_sq, UNPLACED),
            GOD_POLYPHEMUS | GOD_THESEUS => {
                self.set_steps(0);
                !used && self.enter_stage(STAGE_END, moved_sq, UNPLACED)
            }
            _ => false,
        };
        if !entered {
            self.end_turn(p);
        }
    }

    /// Terminal win by `p` mid-turn. Clears sub-turn fields so terminal states are canonical
    /// (otherwise a stale `active_worker` made D4 images order-dependent — found by the
    /// symmetry proptest on an Artemis second-step win).
    fn finish_win(&mut self, p: usize) {
        self.sort_workers();
        self.sub_phase = PHASE_TURN;
        self.active_worker = 0;
        self.prev_square = UNPLACED;
        self.stage = STAGE_NONE;
        self.status_flags &= !STEP_MASK;
        for t in &mut self.tags[p] {
            *t &= !(TAG_MOVED | TAG_BUILT);
        }
        self.game_over = true;
        self.winner = p as i8;
    }

    pub(crate) fn end_turn(&mut self, p: usize) {
        self.sub_phase = PHASE_TURN;
        self.active_worker = 0;
        self.prev_square = UNPLACED;
        self.stage = STAGE_NONE;
        self.status_flags &= !STEP_MASK;
        for t in &mut self.tags[p] {
            *t &= !(TAG_MOVED | TAG_BUILT);
        }
        self.sort_workers();
        if self.wins_at_turn_end(p) {
            return self.finish_win(p);
        }
        self.current_player ^= 1;
        if PROBING.with(|c| c.get()) > 0 {
            return; // hypothetical: the opponent's turn is never examined
        }
        self.begin_turn(1 - p);
        // When `begin_turn(1 - p)` sets `TF_PERSEPHONE`, `exists_up_turn()` just proved that
        // `1 - p` has at least one complete legal climbing turn (and Persephone cannot also be
        // Aphrodite), so re-probing `has_valid_action()` on every Persephone ply is redundant.
        if self.turn_flags & (1 << (TF_PERSEPHONE + 1 - p)) == 0 && !self.has_valid_action() {
            self.game_over = true;
            self.winner = p as i8;
        }
    }

    /// Does the side to move have an end-of-turn win condition?
    pub fn has_end_of_turn_win(&self) -> bool {
        matches!(self.gods[self.current_player as usize], GOD_CHRONUS | GOD_MAENADS | GOD_EROS)
    }

    /// Is any (raw) move step still available in this turn? No stage re-enables moves after a
    /// move-less state, so `false` means the rest of the turn only builds / targets / passes.
    pub fn can_still_move(&self) -> bool {
        let mut any = false;
        self.gen_raw(&mut |a| {
            any = matches!(decode_action(a), ActionKind::Turn { .. } | ActionKind::MoveOnly { .. } | ActionKind::Zeus { .. });
            !any
        });
        any
    }

    /// Chronus / Maenads / Eros end-of-turn wins.
    fn wins_at_turn_end(&self, p: usize) -> bool {
        let mine = self.workers[p];
        match self.gods[p] {
            GOD_CHRONUS => (0..NUM_SQUARES).filter(|&t| self.height[t] == DOME && self.low_domes & (1 << t) == 0).count() >= 4,
            GOD_MAENADS => self.workers[1 - p].iter().any(|&o| {
                o != UNPLACED
                    && (0..4).any(|d| {
                        let (a, b) = (NEIGHBORS[o as usize][d], NEIGHBORS[o as usize][d + 4]);
                        a != OFF_BOARD && b != OFF_BOARD && mine.contains(&a) && mine.contains(&b)
                    })
            }),
            GOD_EROS => {
                mine.iter().all(|&w| w != UNPLACED && self.height[w as usize] == 1) && NEIGHBORS[mine[0] as usize].contains(&mine[1])
            }
            _ => false,
        }
    }

    /// Start-of-turn bookkeeping for `p`: Hypnus, Aphrodite, Persephone.
    pub(crate) fn begin_turn(&mut self, p: usize) {
        self.turn_flags &= !((1 << p) | (1 << (TF_DOWN + p)) | (1 << (TF_PERSEPHONE + p)));
        for t in &mut self.tags[p] {
            *t &= !TRANSIENT_TAGS;
        }
        let (mine, theirs) = (self.workers[p], self.workers[1 - p]);
        match self.gods[1 - p] {
            GOD_HYPNUS if mine.iter().all(|&w| w != UNPLACED) => {
                let (h0, h1) = (self.height[mine[0] as usize], self.height[mine[1] as usize]);
                if h0 != h1 {
                    self.tags[p][(h1 > h0) as usize] |= TAG_FROZEN;
                }
            }
            GOD_APHRODITE => {
                for i in 0..2 {
                    let w = mine[i];
                    if w != UNPLACED && theirs.iter().any(|&o| o != UNPLACED && NEIGHBORS[w as usize].contains(&o)) {
                        self.tags[p][i] |= TAG_APHRODITE;
                    }
                }
            }
            GOD_PERSEPHONE if self.sub_phase >= PHASE_TURN => {
                let saved = (self.current_player, self.sub_phase, self.stage);
                self.current_player = p as u8;
                self.sub_phase = PHASE_TURN;
                self.stage = STAGE_NONE;
                if probing(|| self.exists_up_turn()) {
                    self.turn_flags |= 1 << (TF_PERSEPHONE + p);
                }
                (self.current_player, self.sub_phase, self.stage) = saved;
            }
            _ => {}
        }
    }

    /// Does the side to move have some complete turn that moves up (Persephone's "if possible")?
    fn exists_up_turn(&self) -> bool {
        let p = self.current_player as usize;
        // With no climbing move left (including Hermes level steps and Prometheus pre-built
        // turns, which never climb), `turn_flags & (1 << p)` is fixed for the rest of the turn.
        if self.stage != STAGE_NONE
            && (matches!(self.stage, STAGE_HERMES | STAGE_PROMETHEUS) || !self.can_still_move())
        {
            return self.turn_flags & (1 << p) != 0 && self.has_valid_action();
        }
        if self.stage == STAGE_CHAIN {
            let mut visited = [[0u32; 2]; (CHAIN_MAX_STEPS + 1) as usize];
            return self.exists_up_turn_in_chain(&mut visited);
        }
        let already_up = self.turn_flags & (1 << p) != 0;
        let mut chain_vis = [[[0u32; 2]; (CHAIN_MAX_STEPS + 1) as usize]; 2];
        let mut found = false;
        self.gen_raw(&mut |a| {
            // Single-move `Turn` and `Zeus` actions never enter a stage that moves again, and
            // every optional stage they enter offers `PASS_ACTION`, so their climb outcome is
            // known in `O(1)` without cloning `*self` or running `apply_action_gods`.
            match decode_action(a) {
                ActionKind::Turn { w, m, .. } | ActionKind::Zeus { w, m } => {
                    found = already_up || self.move_step_climbs(p, w, m);
                    return !found;
                }
                ActionKind::Force { .. } if !already_up => return true,
                ActionKind::BuildOnly { .. }
                    if !(already_up || (self.stage == STAGE_NONE && self.gods[p] == GOD_ACHILLES)) =>
                {
                    return true;
                }
                _ => {}
            }
            let mut c = *self;
            c.apply_action_gods(a);
            found = if c.game_over || c.current_player as usize != p {
                c.turn_flags & (1 << p) != 0 && !(c.game_over && c.winner != p as i8)
            } else if self.stage == STAGE_NONE && c.stage == STAGE_CHAIN {
                let ActionKind::MoveOnly { w, .. } = decode_action(a) else { unreachable!() };
                c.exists_up_turn_in_chain(&mut chain_vis[w])
            } else {
                c.exists_up_turn()
            };
            !found
        });
        found
    }

    /// Memoizes `(steps, moved_up, active_sq)` across `STAGE_CHAIN` (`Triton` / `Atalanta`),
    /// where board heights, opponent workers, the unmoved worker, tags, and `power_used` are
    /// invariant across `MoveOnly` steps (capping chain probes to `<= 50` states instead of `8^4`).
    fn exists_up_turn_in_chain(&self, visited: &mut [[u32; 2]; (CHAIN_MAX_STEPS + 1) as usize]) -> bool {
        let p = self.current_player as usize;
        let (god, og) = (self.gods[p], self.gods[1 - p]);
        let steps = self.steps() as usize;
        let up_bit = ((self.turn_flags >> p) & 1) as usize;
        if up_bit == 1 && og != GOD_LIMUS && (god != GOD_ATALANTA || steps >= 1) {
            return true;
        }
        if up_bit == 0 && steps == CHAIN_MAX_STEPS as usize {
            return false;
        }
        let sq = self.workers[p][self.active_worker as usize] as usize;
        let mask = 1u32 << sq;
        if visited[steps][up_bit] & mask != 0 {
            return false;
        }
        visited[steps][up_bit] |= mask;
        let mut found = false;
        self.gen_raw(&mut |a| {
            if up_bit == 0 && a >= BUILD_ONLY_OFFSET {
                return true;
            }
            let mut c = *self;
            c.apply_action_gods(a);
            found = if c.game_over || c.current_player as usize != p {
                c.turn_flags & (1 << p) != 0 && !(c.game_over && c.winner != p as i8)
            } else if c.stage == STAGE_CHAIN {
                c.exists_up_turn_in_chain(visited)
            } else {
                c.exists_up_turn()
            };
            !found
        });
        found
    }

    /// Placement legality beyond "unoccupied" (Eros: opposite edges).
    pub(crate) fn placement_ok(&self, sq: usize) -> bool {
        let p = self.current_player as usize;
        if self.gods[p] != GOD_EROS {
            return true;
        }
        if self.sub_phase == PHASE_PLACE_0 {
            return is_perimeter(sq);
        }
        let e = edges(self.workers[p][0] as usize);
        let opp = ((e << 2) | (e >> 2)) & 0b1111;
        opp & edges(sq) != 0
    }

    /// Exact "can `player` win during their turn" for all god games (Mortal games use the
    /// bitboard fast path in `has_winning_move_for`).
    pub(crate) fn has_winning_move_gods(&self, player: usize) -> bool {
        if self.current_player as usize == player {
            return self.can_win_this_turn_entry();
        }
        // Fast board-level precondition before cloning or invoking `begin_turn(player)` (which
        // runs recursive `exists_up_turn` probing whenever Persephone is on the board): transient
        // turn tags (`TF_PERSEPHONE`, `TAG_FROZEN`, `TAG_APHRODITE`) never create L3 towers, raise
        // worker heights, or lift Athena's climb restriction.
        if !self.can_player_win_board_precond(player, STAGE_NONE) {
            return false;
        }
        // When probing the non-moving side from inside a `PHASE_SUB_TURN` node, strip the
        // mover's unfinished sub-turn state so `player`'s hypothetical turn starts cleanly at
        // `PHASE_TURN` / `STAGE_NONE`.
        let mut s = *self;
        s.sub_phase = PHASE_TURN;
        s.active_worker = 0;
        s.prev_square = UNPLACED;
        s.stage = STAGE_NONE;
        s.status_flags &= !STEP_MASK;
        for t in &mut s.tags[1 - player] {
            *t &= !(TAG_MOVED | TAG_BUILT);
        }
        s.current_player = player as u8;
        if matches!(s.gods[1 - player], GOD_HYPNUS | GOD_APHRODITE | GOD_PERSEPHONE) {
            s.begin_turn(player);
        } else {
            s.turn_flags &= !((1 << player) | (1 << (TF_DOWN + player)) | (1 << (TF_PERSEPHONE + player)));
            s.tags[player][0] &= !TRANSIENT_TAGS;
            s.tags[player][1] &= !TRANSIENT_TAGS;
        }
        s.can_win_this_turn_entry()
    }

    /// Entry point that defers entering TLS `probing` until a state transition is actually needed.
    #[inline]
    fn can_win_this_turn_entry(&self) -> bool {
        self.can_win_this_turn()
    }

    /// Does any non-Hera-blocked level-3 tower currently stand on the board?
    #[inline]
    fn has_winnable_l3(&self, p: usize) -> bool {
        let hera = self.gods[1 - p] == GOD_HERA;
        (0..NUM_SQUARES).any(|t| self.height[t] == 3 && !(hera && is_perimeter(t)))
    }

    /// Necessary board/height precondition for `p` to win from `stage` during a single turn.
    #[inline]
    fn can_player_win_board_precond(&self, p: usize, stage: u8) -> bool {
        let god = self.gods[p];
        let restricted = self.athena_restricts(p);
        let can_climb = !restricted && self.has_winnable_l3(p);
        if matches!(god, GOD_CHRONUS | GOD_MAENADS | GOD_EROS) {
            let can_eot = match god {
                GOD_CHRONUS => {
                    self.height.contains(&3)
                        || (0..NUM_SQUARES)
                            .filter(|&t| self.height[t] == DOME && self.low_domes & (1 << t) == 0)
                            .count()
                            >= 4
                }
                GOD_EROS => {
                    self.workers[p].iter().all(|&w| w != UNPLACED)
                        && self.workers[p].iter().any(|&w| self.height[w as usize] == 1)
                }
                GOD_MAENADS => {
                    self.workers[p].iter().all(|&w| w != UNPLACED)
                        && self.workers[p].iter().any(|&w| {
                            self.workers[1 - p]
                                .iter()
                                .any(|&o| o != UNPLACED && NEIGHBORS[o as usize].contains(&w))
                        })
                }
                _ => true,
            };
            return can_climb || can_eot;
        }
        if god == GOD_PAN {
            return can_climb || self.workers[p].iter().any(|&w| w != UNPLACED && self.height[w as usize] >= 2);
        }
        if restricted {
            return false;
        }
        let achilles_build_win = stage == STAGE_NONE
            && god == GOD_ACHILLES
            && !self.used(p)
            && self.workers[p].iter().any(|&w| w != UNPLACED && self.height[w as usize] == 2)
            && {
                let hera = self.gods[1 - p] == GOD_HERA;
                (0..NUM_SQUARES)
                    .any(|t| (self.height[t] == 2 || self.height[t] == 3) && !(hera && is_perimeter(t)))
            };
        achilles_build_win || can_climb
    }

    /// Exact search over winning continuations of the current turn.
    ///
    /// Why board/height/transposition pruning: alpha-beta calls `has_winning_move` and
    /// `has_winning_move_for(1 - p)` at every node. Without an L3 tower on the board (and
    /// excluding Pan, Achilles pre-building L2->L3, and satisfied end-of-turn win preconditions),
    /// multi-step gods (`Atalanta`, `Triton`, `Terpsichore`, `Castor`) would otherwise clone
    /// thousands of `MoveOnly` continuations per node (`10,032` applies on a single `Atalanta`
    /// check, cutting alpha-beta to `3.5 kNPS`). Even when an L3 tower exists, `STAGE_CHAIN`
    /// (`Atalanta` / `Triton`) leaves heights and other workers invariant, so memoizing
    /// `(steps, moved_up, active_sq)` plus the `max_up == 1` remaining-height bound reduces the
    /// `8^4` chain tree to a handful of probes.
    fn can_win_this_turn(&self) -> bool {
        let p = self.current_player as usize;
        let god = self.gods[p];
        let restricted = self.athena_restricts(p);
        let stage = self.stage;
        if !self.can_player_win_board_precond(p, stage) {
            return false;
        }
        if self.has_end_of_turn_win() {
            return probing(|| {
                let mut win = false;
                self.for_each_action_gods(&mut |a| {
                    let mut t = *self;
                    t.apply_action_gods(a);
                    win = t.current_player as usize == p
                        && ((t.game_over && t.winner == p as i8) || (!t.game_over && t.can_win_this_turn()));
                    !win
                });
                win
            });
        }

        if stage != STAGE_NONE {
            let can_win_in_stage = match stage {
                STAGE_CHAIN => self.steps() < CHAIN_MAX_STEPS,
                STAGE_ARTEMIS | STAGE_ACHILLES | STAGE_CASTOR_MOVE => true,
                STAGE_TERP => (0..2).any(|i| self.workers[p][i] != UNPLACED && self.tags[p][i] & TAG_MOVED == 0),
                _ => false,
            };
            if !can_win_in_stage {
                return false;
            }
        }

        let has_end_constraint =
            self.gods[1 - p] == GOD_APHRODITE || self.turn_flags & (1 << (TF_PERSEPHONE + p)) != 0;
        let active = self.active_worker as usize;
        let turn_workers: &[usize] = match (stage, god) {
            (STAGE_NONE, GOD_TERPSICHORE) => &[],
            (STAGE_NONE, _) => &[0, 1],
            (STAGE_ARTEMIS | STAGE_ACHILLES, _) => std::slice::from_ref(&active),
            _ => &[],
        };
        let simple_climb = god != GOD_PAN && god != GOD_IRIS;
        if !(simple_climb && restricted) {
            for &w in turn_workers {
                let from = self.workers[p][w];
                if from == UNPLACED {
                    continue;
                }
                let hf = self.height[from as usize];
                if simple_climb && hf != 2 && !(god == GOD_BELLEROPHON && !self.used(p) && hf == 1) {
                    continue;
                }
                for m in 0..8 {
                    if simple_climb {
                        let to = self.nb(p, from as usize, m);
                        if to == OFF_BOARD || self.height[to as usize] != 3 {
                            continue;
                        }
                    }
                    let Some(mv) = self.try_move(p, w, m) else { continue };
                    if stage == STAGE_ARTEMIS && mv.to as u8 == self.prev_square {
                        continue;
                    }
                    if god == GOD_TRITON && is_perimeter(mv.to) {
                        continue; // handled as MoveOnly below
                    }
                    if mv.win {
                        let a = encode_turn_action(w, m, opposite_dir(m));
                        if !has_end_constraint || self.completes_legally(a) {
                            return true;
                        }
                    }
                }
            }
        }

        let can_branch_to_win = match (stage, god) {
            (STAGE_NONE, GOD_ARTEMIS) => !restricted,
            (STAGE_NONE, GOD_TRITON | GOD_TERPSICHORE | GOD_CASTOR) => true,
            (STAGE_NONE, GOD_ATALANTA | GOD_ACHILLES) => !self.used(p),
            (STAGE_CHAIN | STAGE_TERP | STAGE_CASTOR_MOVE, _) => true,
            _ => false,
        };
        if !can_branch_to_win {
            return false;
        }
        if matches!(god, GOD_TERPSICHORE | GOD_CASTOR | GOD_ACHILLES)
            && !(0..2).any(|i| {
                let w = self.workers[p][i];
                w != UNPLACED && self.tags[p][i] & TAG_MOVED == 0 && self.height[w as usize] == 2
            })
        {
            return false;
        }
        probing(|| {
            if stage == STAGE_CHAIN {
                let mut visited = [[0u32; 2]; (CHAIN_MAX_STEPS + 1) as usize];
                return self.can_win_in_chain(&mut visited);
            }
            if stage == STAGE_NONE && god == GOD_ARTEMIS {
                // Artemis wins on her second step iff step 1's final square (`mv.to`, after any
                // Harpies slide) has height 2 with an L3 climb available on step 2.
                let harpies = self.gods[1 - p] == GOD_HARPIES;
                for w in 0..2 {
                    let from = self.workers[p][w];
                    if from == UNPLACED || (!harpies && self.height[from as usize] == 0) {
                        continue;
                    }
                    for m in 0..8 {
                        let to = NEIGHBORS[from as usize][m];
                        if to == OFF_BOARD || (!harpies && self.height[to as usize] != 2) {
                            continue;
                        }
                        let Some(mv) = self.try_move(p, w, m) else { continue };
                        if mv.win || self.height[mv.to] != 2 {
                            continue;
                        }
                        let a = encode_move_only(w, m);
                        if (has_end_constraint || self.gods[1 - p] == GOD_LIMUS) && !self.completes_legally(a) {
                            continue;
                        }
                        let mut t = *self;
                        t.apply_action_gods(a);
                        if t.can_win_this_turn() {
                            return true;
                        }
                    }
                }
                return false;
            }
            let mut chain_vis = [[[0u32; 2]; (CHAIN_MAX_STEPS + 1) as usize]; 2];
            let mut win = false;
            self.for_each_action_gods(&mut |a| {
                if !(MOVE_ONLY_OFFSET..PASS_ACTION).contains(&a) {
                    return true;
                }
                if a >= BUILD_ONLY_OFFSET && !(stage == STAGE_NONE && god == GOD_ACHILLES) {
                    return true;
                }
                let mut t = *self;
                t.apply_action_gods(a);
                win = t.current_player as usize == p
                    && ((t.game_over && t.winner == p as i8)
                        || (!t.game_over
                            && if stage == STAGE_NONE && t.stage == STAGE_CHAIN {
                                let ActionKind::MoveOnly { w, .. } = decode_action(a) else { unreachable!() };
                                t.can_win_in_chain(&mut chain_vis[w])
                            } else {
                                t.can_win_this_turn()
                            }));
                !win
            });
            win
        })
    }

    /// Memoized `STAGE_CHAIN` (`Triton` / `Atalanta`) win probe with `max_up == 1` height pruning.
    fn can_win_in_chain(&self, visited: &mut [[u32; 2]; (CHAIN_MAX_STEPS + 1) as usize]) -> bool {
        let steps = self.steps();
        if steps >= CHAIN_MAX_STEPS {
            return false;
        }
        let p = self.current_player as usize;
        let sq = self.workers[p][self.active_worker as usize] as usize;
        let hf = self.height[sq];
        let min_moves = if hf == 3 { 2 } else { 3 - hf };
        if CHAIN_MAX_STEPS - steps < min_moves {
            return false;
        }
        let up_bit = ((self.turn_flags >> p) & 1) as usize;
        let mask = 1u32 << sq;
        if visited[steps as usize][up_bit] & mask != 0 {
            return false;
        }
        visited[steps as usize][up_bit] |= mask;
        let mut win = false;
        self.for_each_action_gods(&mut |a| {
            if !(MOVE_ONLY_OFFSET..BUILD_ONLY_OFFSET).contains(&a) {
                return true;
            }
            let mut t = *self;
            t.apply_action_gods(a);
            win = t.current_player as usize == p
                && ((t.game_over && t.winner == p as i8)
                    || (!t.game_over && t.stage == STAGE_CHAIN && t.can_win_in_chain(visited)));
            !win
        });
        win
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bruteforce_can_win(s: &CompactSantoriniState) -> bool {
        let p = s.current_player;
        if s.stage != STAGE_NONE && !s.has_end_of_turn_win() && !s.can_still_move() {
            return false;
        }
        let mut win = false;
        s.gen_raw(&mut |a| {
            if !s.completes_legally(a) {
                return true;
            }
            let mut t = *s;
            t.apply_action_gods(a);
            win = t.current_player == p
                && ((t.game_over && t.winner == p as i8) || (!t.game_over && bruteforce_can_win(&t)));
            !win
        });
        win
    }

    fn bruteforce_exists_up(s: &CompactSantoriniState) -> bool {
        let p = s.current_player as usize;
        if s.stage != STAGE_NONE && !s.can_still_move() {
            let mut any = false;
            s.gen_raw(&mut |_| {
                any = true;
                false
            });
            return s.turn_flags & (1 << p) != 0 && any;
        }
        let mut found = false;
        s.gen_raw(&mut |a| {
            let mut c = *s;
            c.apply_action_gods(a);
            found = if c.game_over || c.current_player as usize != p {
                c.turn_flags & (1 << p) != 0 && !(c.game_over && c.winner != p as i8)
            } else {
                bruteforce_exists_up(&c)
            };
            !found
        });
        found
    }

    #[test]
    fn fast_action_filter_and_win_check_match_bruteforce() {
        let mut z = 0xD1CE_5EED_2026_0502u64;
        let mut next = || {
            z ^= z << 13;
            z ^= z >> 7;
            z ^= z << 17;
            z
        };
        let mut buf = [0i32; MAX_ACTIONS];
        for g in 0..1200u64 {
            let gods = if g < 176 {
                // Sweep every supported god on both sides of Persephone explicitly.
                let other = SUPPORTED_GODS[(g as usize) % SUPPORTED_GODS.len()];
                if g % 2 == 0 { [other, GOD_PERSEPHONE] } else { [GOD_PERSEPHONE, other] }
            } else {
                [
                    SUPPORTED_GODS[next() as usize % SUPPORTED_GODS.len()],
                    SUPPORTED_GODS[next() as usize % SUPPORTED_GODS.len()],
                ]
            };
            if gods == [GOD_MORTAL, GOD_MORTAL] {
                continue;
            }
            let mut s = CompactSantoriniState::new_with_gods(gods);
            while !s.game_over {
                if s.sub_phase >= PHASE_TURN {
                    if s.stage == STAGE_NONE && s.gods[1 - s.current_player as usize] == GOD_PERSEPHONE {
                        assert_eq!(
                            probing(|| s.exists_up_turn()),
                            probing(|| bruteforce_exists_up(&s)),
                            "exists_up_turn mismatch in game {g} ({gods:?}): {s:?}"
                        );
                    }
                    let mut fast = Vec::new();
                    s.for_each_action_gods(&mut |a| {
                        fast.push(a);
                        true
                    });
                    let mut full = Vec::new();
                    s.gen_raw(&mut |a| {
                        if s.completes_legally(a) {
                            full.push(a);
                        }
                        true
                    });
                    assert_eq!(fast, full, "fast-path action mismatch in game {g} ({gods:?}): {s:?}");
                    for player in 0..2 {
                        let mut probe = s;
                        if probe.current_player as usize != player {
                            probe.sub_phase = PHASE_TURN;
                            probe.active_worker = 0;
                            probe.prev_square = UNPLACED;
                            probe.stage = STAGE_NONE;
                            probe.status_flags &= !STEP_MASK;
                            for t in &mut probe.tags[1 - player] {
                                *t &= !(TAG_MOVED | TAG_BUILT);
                            }
                            probe.current_player = player as u8;
                            probe.begin_turn(player);
                        }
                        let expected = probing(|| bruteforce_can_win(&probe));
                        assert_eq!(
                            s.has_winning_move_for(player),
                            expected,
                            "has_winning_move_for({player}) mismatch in game {g} ({gods:?}): {s:?}"
                        );
                    }
                }
                let n = s.write_valid_actions_into(&mut buf);
                s.apply_action(buf[next() as usize % n]);
            }
        }
    }

    #[test]
    fn artemis_harpies_slide_onto_level_two_is_detected_as_winning_move() {
        // Artemis already standing on L3 (e.g. pushed there earlier) steps onto adjacent L3,
        // Harpies slides her down onto L2 at (2,2), and step 2 climbs onto L3 at (1,2) to win.
        let mut h = [0u8; NUM_SQUARES];
        h[10] = 3; // (2,0)
        h[11] = 3; // (2,1)
        h[12] = 2; // (2,2)
        h[13] = 3; // (2,3) stops the Harpies slide at (2,2)
        h[7] = 3; // (1,2) winning climb target
        let mut s = CompactSantoriniState::new_with_gods([GOD_ARTEMIS, GOD_HARPIES]);
        s.height = h;
        s.workers = [[10, 24], [0, 4]];
        s.sub_phase = PHASE_TURN;
        s.begin_turn(0);
        assert_eq!(s.has_winning_move(), probing(|| bruteforce_can_win(&s)));
        assert!(s.has_winning_move());
    }

    fn count_probe_applies<T>(f: impl FnOnce() -> T) -> (T, u32) {
        PROBE_APPLIES.with(|c| c.set(0));
        let out = f();
        (out, PROBE_APPLIES.with(|c| c.get()))
    }

    #[test]
    fn can_win_this_turn_prunes_multi_step_chains_and_matches_bruteforce() {
        // 1. Midgame board with no L3 tower: Atalanta, Triton, Terpsichore, Castor, Chronus,
        //    Eros (no L1 worker), and Maenads (not adjacent to opponent) must exit in O(1)
        //    with zero hypothetical state transitions.
        let no_l3 = [
            0, 1, 2, 1, 0, //
            1, 2, 0, 2, 1, //
            2, 0, 1, 0, 2, //
            1, 2, 0, 2, 1, //
            0, 1, 2, 1, 0,
        ];
        for god in [GOD_ATALANTA, GOD_TRITON, GOD_TERPSICHORE, GOD_CASTOR, GOD_CHRONUS] {
            let s = CompactSantoriniState::from_position_with_gods(no_l3, [[6, 18], [8, 16]], 0, [god, GOD_MORTAL]);
            let (win, applies) = count_probe_applies(|| s.has_winning_move());
            assert!(!win && !probing(|| bruteforce_can_win(&s)), "unexpected win for {god}");
            assert_eq!(applies, 0, "expected 0 probe applies on no-L3 board for god {god}, got {applies}");
        }
        // Achilles with neither worker on L2 cannot climb to L3 even after pre-building an L2->L3.
        let s_ach_low = CompactSantoriniState::from_position_with_gods(no_l3, [[0, 24], [8, 16]], 0, [GOD_ACHILLES, GOD_MORTAL]);
        assert_eq!(count_probe_applies(|| s_ach_low.has_winning_move()), (false, 0));

        // 2. Board WITH a reachable/unreachable L3 tower: Atalanta chain memoization + height
        //    bound must keep probe applies bounded (<= 80 instead of thousands) while matching
        //    bruteforce across tricky opponent gods and starting heights (including hf == 3).
        let with_l3 = [
            3, 2, 1, 0, 0, //
            2, 1, 0, 0, 0, //
            1, 0, 0, 0, 0, //
            0, 0, 0, 1, 2, //
            0, 0, 1, 2, 3,
        ];
        for opp in [GOD_MORTAL, GOD_HERA, GOD_HARPIES, GOD_APHRODITE, GOD_PERSEPHONE, GOD_LIMUS, GOD_ATHENA] {
            for workers in [[[12, 22], [2, 17]], [[0, 12], [2, 17]], [[6, 18], [7, 17]]] {
                for god in [GOD_ATALANTA, GOD_TRITON, GOD_TERPSICHORE, GOD_CASTOR, GOD_ACHILLES] {
                    let mut s = CompactSantoriniState::new_with_gods([god, opp]);
                    s.height = with_l3;
                    s.workers = workers;
                    s.sub_phase = PHASE_TURN;
                    if opp == GOD_ATHENA {
                        s.status_flags |= 1 << 1;
                    }
                    s.begin_turn(0);
                    let expected = probing(|| bruteforce_can_win(&s));
                    let (got, applies) = count_probe_applies(|| s.has_winning_move());
                    assert_eq!(got, expected, "mismatch for god={god} vs opp={opp} workers={workers:?}");
                    if god == GOD_ATALANTA && opp == GOD_MORTAL {
                        assert!(applies <= 80, "Atalanta probe explosion ({applies} applies) on {workers:?}");
                    }
                }
            }
        }

        // 3. Achilles with zero L3 squares MUST still detect building an L2 -> L3 and climbing.
        let mut ach = [0u8; NUM_SQUARES];
        ach[6] = 2; // Achilles stands at L2
        ach[7] = 2; // adjacent L2 that Achilles can pre-build to L3 and climb
        let s = CompactSantoriniState::from_position_with_gods(ach, [[6, 24], [0, 4]], 0, [GOD_ACHILLES, GOD_MORTAL]);
        assert_eq!(s.has_winning_move(), probing(|| bruteforce_can_win(&s)));
        assert!(s.has_winning_move(), "Achilles pre-build L2->L3 win missed");

        // 4. End-of-turn win corner cases: Chronus (3 complete + L3 vs no L3), Eros (L1 pair),
        //    Maenads (sandwich).
        let mut chr = [0u8; NUM_SQUARES];
        chr[0] = DOME;
        chr[1] = DOME;
        chr[2] = DOME;
        let s_no = CompactSantoriniState::from_position_with_gods(chr, [[6, 18], [8, 16]], 0, [GOD_CHRONUS, GOD_MORTAL]);
        assert_eq!(count_probe_applies(|| s_no.has_winning_move()), (false, 0));
        chr[7] = 3;
        let s_yes = CompactSantoriniState::from_position_with_gods(chr, [[6, 18], [8, 16]], 0, [GOD_CHRONUS, GOD_MORTAL]);
        assert!(s_yes.has_winning_move() && probing(|| bruteforce_can_win(&s_yes)));

        let mut eros = [0u8; NUM_SQUARES];
        eros[6] = 1;
        eros[7] = 1;
        let s_eros = CompactSantoriniState::from_position_with_gods(eros, [[6, 12], [0, 24]], 0, [GOD_EROS, GOD_MORTAL]);
        assert!(s_eros.has_winning_move() && probing(|| bruteforce_can_win(&s_eros)));
        let s_eros_no = CompactSantoriniState::from_position_with_gods([0u8; NUM_SQUARES], [[6, 12], [0, 24]], 0, [GOD_EROS, GOD_MORTAL]);
        assert_eq!(count_probe_applies(|| s_eros_no.has_winning_move()), (false, 0));

        let s_mae_far = CompactSantoriniState::from_position_with_gods([0u8; NUM_SQUARES], [[0, 4], [20, 24]], 0, [GOD_MAENADS, GOD_MORTAL]);
        assert_eq!(count_probe_applies(|| s_mae_far.has_winning_move()), (false, 0));
        let s_mae_win = CompactSantoriniState::from_position_with_gods([0u8; NUM_SQUARES], [[11, 17], [12, 24]], 0, [GOD_MAENADS, GOD_MORTAL]);
        assert!(s_mae_win.has_winning_move() && probing(|| bruteforce_can_win(&s_mae_win)));
    }
}
