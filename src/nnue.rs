//! `SANTNNUE03`: quantized, pantheon-aware NNUE evaluator for Santorini.
//!
//! Why NNUE: alpha-beta needs millions of leaf evaluations per second, which a PyTorch
//! ResNet (~7k evals/s on this CPU) cannot provide. A sparse first layer whose
//! accumulator is updated incrementally makes each leaf cost a few hundred nanoseconds.
//!
//! # Features (per perspective `p`, i.e. "mine" = player `p`), 49 core + 0..=6 sparse of 2,561
//! * `F_SQUARE`          25 squares x 5 heights x 3 occupants (empty / mine / theirs) — 25 active.
//! * `F_PAIR`            Chebyshev distance d in 1..=4: #(mine, theirs) pairs in 0..=4 — 4 active.
//! * `F_THREAT`          per side: Base unconstrained # winning squares (0..=2+), # climbs onto L2+ (0..=3+) — 4 active.
//! * `F_MY_GOD`          my dense god index in `0..44` — 1 active (dense main-effect gradient).
//! * `F_OPP_GOD`         opponent dense god index in `0..44` — 1 active.
//! * `F_GOD_PAIR`        `(my_god, opp_god)` ordered pair in `0..44*44` — 1 active (matchup interaction).
//! * `F_SETUP`           setup (`F_SETUP`) vs play (`F_NOSETUP`) — 1 active.
//! * `F_STAGE`           unified sub-turn stage: `0` (`STAGE_NONE`), `1..=16` (mine), `17..=32` (theirs) — 1 active.
//! * `F_ATHENA`          Athena climb restriction: `me + 2 * opp` — 1 active.
//! * `F_POWER_USED`      once-per-game power spent: `me + 2 * opp` — 1 active.
//! * `F_TURN_FLAGS`      mover's `(moved_up, moved_up_down, persephone)` mask x `(mine / theirs)` — 1 active.
//! * `F_STEPS`           chain / multi-step counter `0..=4` — 1 active.
//! * `F_WORKER_COUNT`    placed worker counts `my_count * 3 + opp_count` (Bia / Medusa / Theseus) — 1 active.
//! * `F_MY_FEMALE`       my female worker state (`0` = none, `1..=4` = height `0..=3`) — 1 active.
//! * `F_OPP_FEMALE`      opponent female worker state (`0` = none, `1..=4` = height `0..=3`) — 1 active.
//! * `F_MOVER_TAGS`      mover `(any_frozen x aphrodite_count)` x `(mine / theirs)` — 1 active.
//! * `F_SUB_PROGRESS`    mover `(any_moved + 2 * any_built)` x `(mine / theirs)` — 1 active.
//! * `F_COMPLETE_TOWERS` complete 4-piece towers (`0..=5+`, for Chronus) — 1 active.
//! * `F_LOW_DOMES`       incomplete low domes (`0..=4+`, Atlas / Heracles / Selene) — 1 active.
//! * `F_MY_GOD_WINS`     sparse (`1` or `2+`) god-legal immediate move-win targets (Hera, Athena, Hypnus, Pan/Hades, Bellerophon, Urania, Iris, Hippolyta, Minotaur) — 0..=1 active.
//! * `F_OPP_GOD_WINS`    sparse (`1` or `2+`) opponent god-legal immediate move-win targets — 0..=1 active.
//! * `F_MY_GOD_CLIMBS`   sparse (`1`, `2`, `3+`) god-legal climbs onto L2+ — 0..=1 active.
//! * `F_OPP_GOD_CLIMBS`  sparse (`1`, `2`, `3+`) opponent god-legal climbs onto L2+ — 0..=1 active.
//! * `F_MY_GOD_TACTIC`   sparse (`1` or `2+`) imminent 1-turn alt-win/kill (`Chronus`, `Achilles`, `Eros`, `Maenads`, `Medusa`, `Theseus`, `Bia`) — 0..=1 active.
//! * `F_OPP_GOD_TACTIC`  sparse (`1` or `2+`) opponent imminent 1-turn alt-win/kill — 0..=1 active.
//!
//! Why sparse (`count >= 1` only) for `2547..2561`: a `count == 0` bin is linearly collinear with
//! `ft_b` (`I(c=0) = 1 - sum_{k>=1} I(c=k)`), wastes 4 accumulator vector ops on every quiet node,
//! and perturbs baseline open-board gradients during warm-start distillation. Appending `2547..2561`
//! after the 49 fixed `SANTNNUE02` slots keeps feature IDs `0..2547` unchanged, so `Nnue::from_bytes`
//! loads `SANTNNUE02` checkpoints with zeroed trailing rows `2547..2561` bit-identically.
//!
//! # Network
//! `acc_p = ft_b + sum(ft_w[f])` (i16, scale `QA`) for both perspectives; output bucket
//! `b` (by blocks built) computes `sum(crelu(acc_stm) * w[b][..H] + crelu(acc_nstm) *
//! w[b][H..]) + out_b[b]` at scale `QA * QB`. The float result is a logit;
//! `tanh(logit)` is the side-to-move value in `[-1, 1]`.
//!
//! # Incremental update (`NnueAccCache`)
//! Each search thread keeps the last feature list + accumulator per perspective and
//! applies only the slot-wise difference on the 49 fixed core slots plus any trailing sparse
//! tactical slots (`49..len`).

use crate::compact::*;
use crate::gods::{
    god_slot, is_perimeter, CHAIN_MAX_STEPS, GOD_ACHILLES, GOD_BELLEROPHON, GOD_BIA, GOD_CHRONUS, GOD_EROS, GOD_HADES,
    GOD_HERA, GOD_HIPPOLYTA, GOD_HYPNUS, GOD_IRIS, GOD_LIMUS, GOD_MAENADS, GOD_MEDUSA, GOD_MINOTAUR, GOD_PAN,
    GOD_PEGASUS, GOD_THESEUS, GOD_URANIA, NUM_STAGES, NUM_SUPPORTED_GODS, TAG_APHRODITE, TAG_BUILT, TAG_FEMALE,
    TAG_FROZEN, TAG_MOVED, TF_DOWN, TF_PERSEPHONE, WRAP_NEIGHBORS,
};

pub const NNUE_MAGIC: &[u8; 10] = b"SANTNNUE03";
pub const NNUE_VERSION: u32 = 3;
pub const NNUE_MAGIC_V2: &[u8; 10] = b"SANTNNUE02";
pub const NNUE_VERSION_V2: u32 = 2;
pub const NNUE_HIDDEN: usize = 256;
pub const NNUE_BUCKETS: usize = 4;
// QA = 255 (accumulator scale) and QB = 512 (output-weight scale; output weights are i16,
// not i8, so they can afford the precision). Training clamps float weights to
// |ft_w| <= 2 and |out_w| <= 8, bounding |acc| <= 54 * 510 ~= 27.5k (i16) and the output sum
// <= 255 * 4096 * 512 ~= 5.3e8 (i32).
pub const QA: i32 = 255;
pub const QB: i32 = 512;

pub const F_SQUARE: usize = 0;
pub const F_PAIR: usize = F_SQUARE + NUM_SQUARES * 5 * 3; // 375
pub const F_THREAT: usize = F_PAIR + 4 * 5; // 395
pub const F_MY_GOD: usize = F_THREAT + 2 * 7; // 409
pub const F_OPP_GOD: usize = F_MY_GOD + NUM_SUPPORTED_GODS; // 453
pub const F_GOD_PAIR: usize = F_OPP_GOD + NUM_SUPPORTED_GODS; // 497
pub const F_SETUP: usize = F_GOD_PAIR + NUM_SUPPORTED_GODS * NUM_SUPPORTED_GODS; // 2433
pub const F_NOSETUP: usize = F_SETUP + 1; // 2434
pub const F_STAGE: usize = F_NOSETUP + 1; // 2435..2468
pub const F_ATHENA: usize = F_STAGE + 1 + 2 * (NUM_STAGES - 1); // 2468..2472
pub const F_POWER_USED: usize = F_ATHENA + 4; // 2472..2476
pub const F_TURN_FLAGS: usize = F_POWER_USED + 4; // 2476..2492
pub const F_STEPS: usize = F_TURN_FLAGS + 16; // 2492..2497
pub const F_WORKER_COUNT: usize = F_STEPS + (CHAIN_MAX_STEPS as usize + 1); // 2497..2506
pub const F_MY_FEMALE: usize = F_WORKER_COUNT + 9; // 2506..2511
pub const F_OPP_FEMALE: usize = F_MY_FEMALE + 5; // 2511..2516
pub const F_MOVER_TAGS: usize = F_OPP_FEMALE + 5; // 2516..2528
pub const F_SUB_PROGRESS: usize = F_MOVER_TAGS + 12; // 2528..2536
pub const F_COMPLETE_TOWERS: usize = F_SUB_PROGRESS + 8; // 2536..2542
pub const F_LOW_DOMES: usize = F_COMPLETE_TOWERS + 6; // 2542..2547
pub const NNUE_FEATURES_V2: usize = F_LOW_DOMES + 5; // 2547
pub const F_MY_GOD_WINS: usize = NNUE_FEATURES_V2; // 2547..2549 (sparse: 1, 2+)
pub const F_OPP_GOD_WINS: usize = F_MY_GOD_WINS + 2; // 2549..2551 (sparse: 1, 2+)
pub const F_MY_GOD_CLIMBS: usize = F_OPP_GOD_WINS + 2; // 2551..2554 (sparse: 1, 2, 3+)
pub const F_OPP_GOD_CLIMBS: usize = F_MY_GOD_CLIMBS + 3; // 2554..2557 (sparse: 1, 2, 3+)
pub const F_MY_GOD_TACTIC: usize = F_OPP_GOD_CLIMBS + 3; // 2557..2559 (sparse: 1, 2+ alt-win/kill)
pub const F_OPP_GOD_TACTIC: usize = F_MY_GOD_TACTIC + 2; // 2559..2561 (sparse: 1, 2+ alt-win/kill)
pub const NNUE_FEATURES: usize = F_OPP_GOD_TACTIC + 2; // 2561
pub const MAX_ACTIVE: usize = 56; // 49 core + up to 6 sparse tactical slots + >=1 padding slot

const HEADER_BYTES: usize = 10 + 6 * 4;

#[derive(Clone, Copy, Debug)]
pub struct FeatureList {
    idx: [u16; MAX_ACTIVE],
    len: u8,
}

impl Default for FeatureList {
    fn default() -> Self {
        Self { idx: [0; MAX_ACTIVE], len: 0 }
    }
}

impl FeatureList {
    #[inline(always)]
    pub fn as_slice(&self) -> &[u16] {
        &self.idx[..self.len as usize]
    }
}

/// Chebyshev distance between squares (compile-time table).
const CHEB: [[u8; NUM_SQUARES]; NUM_SQUARES] = {
    let mut t = [[0u8; NUM_SQUARES]; NUM_SQUARES];
    let mut a = 0;
    while a < NUM_SQUARES {
        let mut b = 0;
        while b < NUM_SQUARES {
            let dr = (a / 5) as i32 - (b / 5) as i32;
            let dc = (a % 5) as i32 - (b % 5) as i32;
            let (dr, dc) = (if dr < 0 { -dr } else { dr }, if dc < 0 { -dc } else { dc });
            t[a][b] = if dr > dc { dr as u8 } else { dc as u8 };
            b += 1;
        }
        a += 1;
    }
    t
};

/// Perimeter bitmask (`r in {0, 4} || c in {0, 4}`, 16 outer squares; D4-invariant).
const PERIMETER_MASK: u32 = {
    let mut m = 0u32;
    let mut sq = 0;
    while sq < NUM_SQUARES {
        if is_perimeter(sq) {
            m |= 1 << sq;
        }
        sq += 1;
    }
    m
};

/// Diagonal-only standard neighbours (`NE, SE, SW, NW`, for Hippolyta; D4-invariant).
const DIAG_NEIGHBOR_MASK: [u32; NUM_SQUARES] = {
    let mut t = [0u32; NUM_SQUARES];
    let mut sq = 0;
    while sq < NUM_SQUARES {
        let mut d = 1;
        while d < 8 {
            let nb = NEIGHBORS[sq][d];
            if nb != OFF_BOARD {
                t[sq] |= 1 << nb;
            }
            d += 2;
        }
        sq += 1;
    }
    t
};

/// Toroidal wrap neighbours (all 8 directions for Urania; D4-invariant).
const WRAP_NEIGHBOR_MASK: [u32; NUM_SQUARES] = {
    let mut t = [0u32; NUM_SQUARES];
    let mut sq = 0;
    while sq < NUM_SQUARES {
        let mut d = 0;
        while d < 8 {
            t[sq] |= 1 << WRAP_NEIGHBORS[sq][d];
            d += 1;
        }
        sq += 1;
    }
    t
};

/// Toroidal diagonal-only wrap neighbours (Urania vs Hippolyta; D4-invariant).
const WRAP_DIAG_NEIGHBOR_MASK: [u32; NUM_SQUARES] = {
    let mut t = [0u32; NUM_SQUARES];
    let mut sq = 0;
    while sq < NUM_SQUARES {
        let mut d = 1;
        while d < 8 {
            t[sq] |= 1 << WRAP_NEIGHBORS[sq][d];
            d += 2;
        }
        sq += 1;
    }
    t
};

/// Base unconstrained (# immediate winning squares, # up-moves onto level >= 2) for `player`.
/// Preserved verbatim in slots `29..33` (`F_THREAT`) so `SANTNNUE02` checkpoints stay bit-exact.
#[inline(always)]
fn threat_counts(s: &CompactSantoriniState, player: usize, free: u32, lv: &[u32; 5]) -> (usize, usize) {
    let mut climbs = 0u32;
    let mut win_mask = 0u32;
    for &w in &s.workers[player] {
        if w == UNPLACED {
            continue;
        }
        let hw = s.height[w as usize] as usize;
        if hw == 0 || hw >= 3 {
            continue; // ground climbs onto level 1 are not counted; nothing above level 3
        }
        let up = NEIGHBOR_MASK[w as usize] & free & lv[hw + 1];
        climbs += up.count_ones();
        if hw == 2 {
            win_mask |= up;
        }
    }
    (win_mask.count_ones() as usize, climbs as usize)
}

/// Pantheon-aware `(eff_wins, god_climbs, alt_tactics)` for `player` in pure `O(1)` bitboard arithmetic.
///
/// * `eff_wins`: distinct immediate move-win landing squares (`0..=2+` after clamping), honoring
///   `Hera` perimeter suppression, `Athena` climb lock, `Hypnus` highest-worker pin (both mover
///   `TAG_FROZEN` and upcoming non-mover pin), `Hippolyta` diagonal restriction, `Urania` torus
///   wrap, `Pan` drop wins (blocked by `Hades`), `Bellerophon` unspent `L1 -> L3` vaults, `Iris`
///   jump-over `L3` wins, and `Minotaur` `L2 -> L3` push-wins.
/// * `god_climbs`: god-legal climbs onto `L2+` (including `Pegasus` / `Bellerophon` `+2` leaps,
///   `Urania` wraps, `Iris` jumps, and `Minotaur` L3 pushes).
/// * `alt_tactics`: imminent 1-turn alt-win / worker-elimination count (`Chronus` 3+ complete
///   towers + open L3, `Achilles` unspent `L2` pre-build mate, `Eros` 1-turn `L1` pair adjacency,
///   `Maenads` 1-turn flank, `Medusa` petrification, `Theseus` unspent assassination, `Bia` ray).
#[inline]
fn god_threat_counts(s: &CompactSantoriniState, player: usize, occ: u32, lv: &[u32; 5]) -> (usize, usize, usize) {
    if s.sub_phase < PHASE_TURN {
        return (0, 0, 0);
    }
    let (god, og) = (s.gods[player], s.gods[1 - player]);
    let stm = (s.current_player & 1) as usize;
    let me = s.workers[player];
    let theirs = s.workers[1 - player];
    let used = (s.power_used & (1 << player)) != 0;
    let restricted = s.athena_restricts(player);
    let free_open = !occ & !lv[4] & 0x01FF_FFFF;
    let win_allowed = if og == GOD_HERA { !PERIMETER_MASK & 0x01FF_FFFF } else { 0x01FF_FFFF };

    // On `player`'s own turn `begin_turn` already recorded `TAG_FROZEN`; while `1 - player` moves,
    // predict Hypnus's start-of-next-turn pin on `player`'s strictly higher worker.
    let hypnus_next = if player != stm && og == GOD_HYPNUS && me[0] != UNPLACED && me[1] != UNPLACED {
        let (h0, h1) = (s.height[me[0] as usize], s.height[me[1] as usize]);
        if h0 > h1 {
            0b01u8
        } else if h1 > h0 {
            0b10u8
        } else {
            0
        }
    } else {
        0
    };

    let mut frozen_w = [false; 2];
    let mut step_masks = [0u32; 2];
    let mut win_mask = 0u32;
    let mut god_climbs = 0u32;

    for wi in 0..2 {
        let w = me[wi];
        if w == UNPLACED {
            frozen_w[wi] = true;
            continue;
        }
        let is_frozen = (s.tags[player][wi] & TAG_FROZEN) != 0 || (hypnus_next & (1 << wi)) != 0;
        frozen_w[wi] = is_frozen;
        if is_frozen {
            continue;
        }
        let sq = w as usize;
        let hw = s.height[sq] as usize;
        let diag_only = (god == GOD_HIPPOLYTA || og == GOD_HIPPOLYTA)
            && !(god == GOD_HIPPOLYTA && (s.tags[player][wi] & TAG_FEMALE) != 0);
        let sm = match (god == GOD_URANIA, diag_only) {
            (false, false) => NEIGHBOR_MASK[sq],
            (false, true) => DIAG_NEIGHBOR_MASK[sq],
            (true, false) => WRAP_NEIGHBOR_MASK[sq],
            (true, true) => WRAP_DIAG_NEIGHBOR_MASK[sq],
        };
        step_masks[wi] = sm;

        if !restricted {
            if hw == 1 {
                let up2 = sm & free_open & lv[2];
                god_climbs += up2.count_ones();
                if god == GOD_BELLEROPHON && !used {
                    let vault3 = sm & free_open & lv[3];
                    god_climbs += vault3.count_ones();
                    win_mask |= vault3 & win_allowed;
                }
            } else if hw == 2 {
                let up3 = sm & free_open & lv[3];
                god_climbs += up3.count_ones();
                win_mask |= up3 & win_allowed;
                if god == GOD_MINOTAUR {
                    for d in 0..8 {
                        if diag_only && d % 2 == 0 {
                            continue;
                        }
                        let to = NEIGHBORS[sq][d];
                        if to != OFF_BOARD && s.height[to as usize] == 3 && (theirs[0] == to || theirs[1] == to) {
                            let push = NEIGHBORS[to as usize][d];
                            if push != OFF_BOARD && ((free_open >> push) & 1) != 0 {
                                god_climbs += 1;
                                win_mask |= (1u32 << to) & win_allowed;
                            }
                        }
                    }
                }
            } else if hw == 0 && ((god == GOD_BELLEROPHON && !used) || god == GOD_PEGASUS) {
                god_climbs += (sm & free_open & lv[2]).count_ones();
            }

            if god == GOD_IRIS {
                for d in 0..8 {
                    if diag_only && d % 2 == 0 {
                        continue;
                    }
                    let mid = NEIGHBORS[sq][d];
                    if mid != OFF_BOARD && ((occ >> mid) & 1) != 0 {
                        let land = NEIGHBORS[mid as usize][d];
                        if land != OFF_BOARD && ((free_open >> land) & 1) != 0 {
                            let hl = s.height[land as usize] as usize;
                            if hl > hw {
                                if hl >= 2 {
                                    god_climbs += 1;
                                }
                                if hl == 3 {
                                    win_mask |= (1u32 << land) & win_allowed;
                                }
                            }
                        }
                    }
                }
            }
        }

        if god == GOD_PAN && og != GOD_HADES && hw >= 2 {
            let low_lv = if hw >= 3 { lv[0] | lv[1] } else { lv[0] };
            win_mask |= sm & free_open & low_lv & win_allowed;
        }
    }

    let mut alt_tactics = 0usize;
    match god {
        GOD_CHRONUS => {
            let complete = (lv[4] & !s.low_domes).count_ones() as usize;
            let open_l3 = lv[3] & free_open;
            if complete >= 4 {
                alt_tactics += 2;
            } else if complete == 3 && open_l3 != 0 {
                alt_tactics += 1;
                let mut reachable = false;
                let mut rem = open_l3;
                while rem != 0 {
                    let t_sq = rem.trailing_zeros() as usize;
                    rem &= rem - 1;
                    for wi in 0..2 {
                        if !frozen_w[wi] && CHEB[me[wi] as usize][t_sq] <= 2 {
                            reachable = true;
                        }
                    }
                }
                if reachable {
                    alt_tactics += 1;
                }
            }
        }
        GOD_ACHILLES if !used && !restricted => {
            let mut limus_adj = 0u32;
            if og == GOD_LIMUS {
                for &o in &theirs {
                    if o != UNPLACED {
                        limus_adj |= NEIGHBOR_MASK[o as usize];
                    }
                }
            }
            for wi in 0..2 {
                if !frozen_w[wi] && s.height[me[wi] as usize] == 2 {
                    let sq = me[wi] as usize;
                    let ach = step_masks[wi] & NEIGHBOR_MASK[sq] & free_open & lv[2] & !limus_adj & win_allowed;
                    alt_tactics += ach.count_ones() as usize;
                }
            }
        }
        GOD_EROS if me[0] != UNPLACED && me[1] != UNPLACED => {
            let (w0, w1) = (me[0] as usize, me[1] as usize);
            let (h0, h1) = (s.height[w0], s.height[w1]);
            if h0 == 1 && h1 == 1 && (NEIGHBOR_MASK[w0] & (1 << w1)) != 0 {
                alt_tactics += 2;
            } else {
                let mut one_turn = false;
                for wi in 0..2 {
                    let anchor = me[1 - wi] as usize;
                    if !frozen_w[wi] && s.height[anchor] == 1 {
                        let hm = s.height[me[wi] as usize];
                        let ok_h = hm <= 2 && !(restricted && 1 > hm) && !(og == GOD_HADES && 1 < hm);
                        if ok_h && (step_masks[wi] & NEIGHBOR_MASK[anchor] & free_open & lv[1]) != 0 {
                            one_turn = true;
                        }
                    }
                }
                if one_turn {
                    alt_tactics += 2;
                } else if (h0 == 1 || h1 == 1) && CHEB[w0][w1] <= 2 {
                    alt_tactics += 1;
                }
            }
        }
        GOD_MAENADS if me[0] != UNPLACED && me[1] != UNPLACED => {
            for &o in &theirs {
                if o == UNPLACED {
                    continue;
                }
                for d in 0..4 {
                    let (s1, s2) = (NEIGHBORS[o as usize][d], NEIGHBORS[o as usize][d + 4]);
                    if s1 == OFF_BOARD || s2 == OFF_BOARD {
                        continue;
                    }
                    if (me[0] == s1 && me[1] == s2) || (me[0] == s2 && me[1] == s1) {
                        alt_tactics += 2;
                        continue;
                    }
                    for (anchor_wi, mover_wi, tgt) in [(0usize, 1usize, s2), (1, 0, s2), (0, 1, s1), (1, 0, s1)] {
                        let expected_anchor = if tgt == s2 { s1 } else { s2 };
                        if me[anchor_wi] == expected_anchor && !frozen_w[mover_wi] && ((free_open >> tgt) & 1) != 0 {
                            let hm = s.height[me[mover_wi] as usize];
                            let ht = s.height[tgt as usize];
                            let ok_step = ((step_masks[mover_wi] >> tgt) & 1) != 0
                                && ht <= hm + 1
                                && !(restricted && ht > hm)
                                && !(og == GOD_HADES && ht < hm);
                            if ok_step {
                                alt_tactics += 2;
                            }
                        }
                    }
                }
            }
        }
        GOD_MEDUSA => {
            for &w in &me {
                if w != UNPLACED {
                    for &o in &theirs {
                        if o != UNPLACED
                            && (NEIGHBOR_MASK[w as usize] & (1 << o)) != 0
                            && s.height[w as usize] > s.height[o as usize]
                        {
                            alt_tactics += 1;
                        }
                    }
                }
            }
        }
        GOD_THESEUS if !used => {
            for &w in &me {
                if w != UNPLACED {
                    for &o in &theirs {
                        if o != UNPLACED
                            && (NEIGHBOR_MASK[w as usize] & (1 << o)) != 0
                            && s.height[o as usize] >= s.height[w as usize] + 2
                        {
                            alt_tactics += 1;
                        }
                    }
                }
            }
        }
        GOD_BIA => {
            for wi in 0..2 {
                if !frozen_w[wi] {
                    let sq = me[wi] as usize;
                    let hw = s.height[sq];
                    for d in 0..8 {
                        let to = NEIGHBORS[sq][d];
                        if to != OFF_BOARD && (((step_masks[wi] & free_open) >> to) & 1) != 0 {
                            let ht = s.height[to as usize];
                            let ok = ht <= hw + 1 && !(restricted && ht > hw) && !(og == GOD_HADES && ht < hw);
                            if ok {
                                let next = NEIGHBORS[to as usize][d];
                                if next != OFF_BOARD && (theirs[0] == next || theirs[1] == next) {
                                    alt_tactics += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    (win_mask.count_ones() as usize, god_climbs as usize, alt_tactics)
}

/// Writes the sorted active-feature lists of `s` for both perspectives in one pass.
///
/// Why one pass: the perspectives share almost everything (square features only swap
/// mine/theirs, pair/tower counts are symmetric, threat/god/pantheon features swap sides).
pub fn write_features_both(s: &CompactSantoriniState, out: &mut [FeatureList; 2]) {
    const SWAP: [usize; 3] = [0, 2, 1];
    let (mut a, mut b) = ([0u16; MAX_ACTIVE], [0u16; MAX_ACTIVE]);
    let mut owner = [0u8; NUM_SQUARES]; // 0 empty, 1 player 0, 2 player 1
    let mut occ = 0u32;
    let mut wc = [0usize; 2];
    let mut fem = [0usize; 2];
    for p in 0..2 {
        for (i, &w) in s.workers[p].iter().enumerate() {
            if w != UNPLACED {
                let sq = w as usize;
                owner[sq] = 1 + p as u8;
                occ |= 1 << sq;
                wc[p] += 1;
                if s.tags[p][i] & TAG_FEMALE != 0 {
                    fem[p] = fem[p].max(1 + (s.height[sq] as usize).min(3));
                }
            }
        }
    }
    for sq in 0..NUM_SQUARES {
        let base = F_SQUARE + sq * 15 + s.height[sq] as usize * 3;
        let o = owner[sq] as usize;
        a[sq] = (base + o) as u16;
        b[sq] = (base + SWAP[o]) as u16;
    }
    let mut n = NUM_SQUARES;
    let mut pair_counts = [0usize; 4];
    for &x in &s.workers[0] {
        for &y in &s.workers[1] {
            if x != UNPLACED && y != UNPLACED {
                pair_counts[CHEB[x as usize][y as usize].min(4) as usize - 1] += 1;
            }
        }
    }
    for (d, &c) in pair_counts.iter().enumerate() {
        a[n] = (F_PAIR + d * 5 + c) as u16;
        b[n] = a[n];
        n += 1;
    }
    let lv = s.level_masks();
    let t = [threat_counts(s, 0, !occ, &lv), threat_counts(s, 1, !occ, &lv)];
    a[n] = (F_THREAT + t[0].0.min(2)) as u16;
    a[n + 1] = (F_THREAT + 3 + t[0].1.min(3)) as u16;
    a[n + 2] = (F_THREAT + 7 + t[1].0.min(2)) as u16;
    a[n + 3] = (F_THREAT + 7 + 3 + t[1].1.min(3)) as u16;
    b[n] = (F_THREAT + t[1].0.min(2)) as u16;
    b[n + 1] = (F_THREAT + 3 + t[1].1.min(3)) as u16;
    b[n + 2] = (F_THREAT + 7 + t[0].0.min(2)) as u16;
    b[n + 3] = (F_THREAT + 7 + 3 + t[0].1.min(3)) as u16;
    n += 4;

    let g = [god_slot(s.gods[0]), god_slot(s.gods[1])];
    a[n] = (F_MY_GOD + g[0]) as u16;
    b[n] = (F_MY_GOD + g[1]) as u16;
    a[n + 1] = (F_OPP_GOD + g[1]) as u16;
    b[n + 1] = (F_OPP_GOD + g[0]) as u16;
    a[n + 2] = (F_GOD_PAIR + g[0] * NUM_SUPPORTED_GODS + g[1]) as u16;
    b[n + 2] = (F_GOD_PAIR + g[1] * NUM_SUPPORTED_GODS + g[0]) as u16;
    n += 3;

    let setup = if s.sub_phase < PHASE_TURN { F_SETUP } else { F_NOSETUP } as u16;
    a[n] = setup;
    b[n] = setup;
    n += 1;

    let stm = (s.current_player & 1) as usize;
    let st = (s.stage as usize).min(NUM_STAGES - 1);
    let tf_bits = (((s.turn_flags >> stm) & 1)
        | (((s.turn_flags >> (TF_DOWN + stm)) & 1) << 1)
        | (((s.turn_flags >> (TF_PERSEPHONE + stm)) & 1) << 2)) as usize;
    let steps_f = (F_STEPS + s.steps().min(CHAIN_MAX_STEPS) as usize) as u16;

    let (mut frozen, mut aph_cnt, mut any_moved, mut any_built) = (0usize, 0usize, 0usize, 0usize);
    for (i, &w) in s.workers[stm].iter().enumerate() {
        if w != UNPLACED {
            let tg = s.tags[stm][i];
            frozen |= (tg & TAG_FROZEN != 0) as usize;
            aph_cnt += (tg & TAG_APHRODITE != 0) as usize;
            any_moved |= (tg & TAG_MOVED != 0) as usize;
            any_built |= (tg & TAG_BUILT != 0) as usize;
        }
    }
    let mtag = frozen * 3 + aph_cnt.min(2);
    let prog = any_moved + 2 * any_built;
    let towers_f = (F_COMPLETE_TOWERS + ((lv[4] & !s.low_domes).count_ones() as usize).min(5)) as u16;
    let low_domes_f = (F_LOW_DOMES + ((lv[4] & s.low_domes).count_ones() as usize).min(4)) as u16;
    let pu = [(s.power_used & 1) as usize, ((s.power_used >> 1) & 1) as usize];

    for (p, out_p) in [(0usize, &mut a), (1usize, &mut b)] {
        let opp = 1 - p;
        let stage_off = if st == 0 {
            0
        } else if p == stm {
            st
        } else {
            (NUM_STAGES - 1) + st
        };
        out_p[n] = (F_STAGE + stage_off) as u16;
        let r = s.athena_restricts(p) as usize + 2 * s.athena_restricts(opp) as usize;
        out_p[n + 1] = (F_ATHENA + r) as u16;
        out_p[n + 2] = (F_POWER_USED + pu[p] + 2 * pu[opp]) as u16;
        out_p[n + 3] = (F_TURN_FLAGS + (if p == stm { 0 } else { 8 }) + tf_bits) as u16;
        out_p[n + 4] = steps_f;
        out_p[n + 5] = (F_WORKER_COUNT + wc[p] * 3 + wc[opp]) as u16;
        out_p[n + 6] = (F_MY_FEMALE + fem[p]) as u16;
        out_p[n + 7] = (F_OPP_FEMALE + fem[opp]) as u16;
        out_p[n + 8] = (F_MOVER_TAGS + (if p == stm { 0 } else { 6 }) + mtag) as u16;
        out_p[n + 9] = (F_SUB_PROGRESS + (if p == stm { 0 } else { 4 }) + prog) as u16;
        out_p[n + 10] = towers_f;
        out_p[n + 11] = low_domes_f;
    }
    n += 12;

    let gt = [god_threat_counts(s, 0, occ, &lv), god_threat_counts(s, 1, occ, &lv)];
    let push_sparse =
        |dst: &mut [u16; MAX_ACTIVE], mut len: usize, my: (usize, usize, usize), opp: (usize, usize, usize)| -> u8 {
            if my.0 > 0 {
                dst[len] = (F_MY_GOD_WINS + my.0.min(2) - 1) as u16;
                len += 1;
            }
            if opp.0 > 0 {
                dst[len] = (F_OPP_GOD_WINS + opp.0.min(2) - 1) as u16;
                len += 1;
            }
            if my.1 > 0 {
                dst[len] = (F_MY_GOD_CLIMBS + my.1.min(3) - 1) as u16;
                len += 1;
            }
            if opp.1 > 0 {
                dst[len] = (F_OPP_GOD_CLIMBS + opp.1.min(3) - 1) as u16;
                len += 1;
            }
            if my.2 > 0 {
                dst[len] = (F_MY_GOD_TACTIC + my.2.min(2) - 1) as u16;
                len += 1;
            }
            if opp.2 > 0 {
                dst[len] = (F_OPP_GOD_TACTIC + opp.2.min(2) - 1) as u16;
                len += 1;
            }
            len as u8
        };
    let len_a = push_sparse(&mut a, n, gt[0], gt[1]);
    let len_b = push_sparse(&mut b, n, gt[1], gt[0]);

    out[0] = FeatureList { idx: a, len: len_a };
    out[1] = FeatureList { idx: b, len: len_b };
}

/// Sorted active-feature list of `s` from perspective `p` (training / tests).
pub fn write_features(s: &CompactSantoriniState, p: usize, out: &mut FeatureList) {
    let mut both = [FeatureList::default(); 2];
    write_features_both(s, &mut both);
    *out = both[p];
}

/// Output bucket by blocks built (game phase).
#[inline(always)]
pub fn bucket(s: &CompactSantoriniState) -> usize {
    let built: u32 = s.height.iter().map(|&h| h as u32).sum();
    (built as usize / 12).min(NNUE_BUCKETS - 1)
}

/// Per-thread incremental accumulator cache (one slot per absolute perspective).
#[derive(Clone, Copy)]
pub struct NnueAccCache {
    feats: [FeatureList; 2],
    acc: [[i16; NNUE_HIDDEN]; 2],
    valid: [bool; 2],
}

impl Default for NnueAccCache {
    fn default() -> Self {
        Self { feats: [FeatureList::default(); 2], acc: [[0; NNUE_HIDDEN]; 2], valid: [false; 2] }
    }
}

#[derive(Clone)]
pub struct Nnue {
    pub ft_w: Vec<i16>, // [NNUE_FEATURES * NNUE_HIDDEN]
    pub ft_b: [i16; NNUE_HIDDEN],
    pub out_w: [[i16; 2 * NNUE_HIDDEN]; NNUE_BUCKETS],
    pub out_b: [i32; NNUE_BUCKETS],
}

impl Nnue {
    pub fn zeros() -> Self {
        Self {
            ft_w: vec![0; NNUE_FEATURES * NNUE_HIDDEN],
            ft_b: [0; NNUE_HIDDEN],
            out_w: [[0; 2 * NNUE_HIDDEN]; NNUE_BUCKETS],
            out_b: [0; NNUE_BUCKETS],
        }
    }

    /// Deterministic random weights (tests / benchmarks).
    pub fn random(seed: u64) -> Self {
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut next = move |range: i32| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % (2 * range as u64 + 1)) as i32 - range
        };
        let mut n = Self::zeros();
        // Draw core `0..NNUE_FEATURES_V2` + head weights first, then `2547..NNUE_FEATURES`,
        // so deterministic test seeds (`Nnue::random(11)`) keep stable head/core weights while
        // still assigning non-zero random weights to every `SANTNNUE03` row.
        n.ft_w[..NNUE_FEATURES_V2 * NNUE_HIDDEN].iter_mut().for_each(|w| *w = next(40) as i16);
        n.ft_b.iter_mut().for_each(|w| *w = next(60) as i16);
        n.out_w.iter_mut().flatten().for_each(|w| *w = next(64) as i16);
        n.out_b.iter_mut().for_each(|w| *w = next(2000));
        n.ft_w[NNUE_FEATURES_V2 * NNUE_HIDDEN..].iter_mut().for_each(|w| *w = next(40) as i16);
        n
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(HEADER_BYTES + self.ft_w.len() * 2 + 4096);
        b.extend_from_slice(NNUE_MAGIC);
        for v in [NNUE_VERSION, NNUE_FEATURES as u32, NNUE_HIDDEN as u32, NNUE_BUCKETS as u32, QA as u32, QB as u32] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        self.ft_w.iter().for_each(|w| b.extend_from_slice(&w.to_le_bytes()));
        self.ft_b.iter().for_each(|w| b.extend_from_slice(&w.to_le_bytes()));
        self.out_w.iter().flatten().for_each(|w| b.extend_from_slice(&w.to_le_bytes()));
        self.out_b.iter().for_each(|w| b.extend_from_slice(&w.to_le_bytes()));
        b
    }

    /// Parses a `SANTNNUE03` (`2,565`-feature) or `SANTNNUE02` (`2,547`-feature, zero-padded
    /// on rows `2547..2565` for 100% bit-exact evaluation) file. Legacy `SANTNNUE01` files must
    /// be converted offline with `scripts/convert_checkpoints.py`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let file_features = if bytes.len() >= HEADER_BYTES && &bytes[..10] == NNUE_MAGIC {
            let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
            let header = [u32_at(10), u32_at(14), u32_at(18), u32_at(22), u32_at(26), u32_at(30)];
            let want = [NNUE_VERSION, NNUE_FEATURES as u32, NNUE_HIDDEN as u32, NNUE_BUCKETS as u32, QA as u32, QB as u32];
            if header != want {
                return Err(format!("SANTNNUE03 header {header:?} unsupported (expected {want:?})"));
            }
            NNUE_FEATURES
        } else if bytes.len() >= HEADER_BYTES && &bytes[..10] == NNUE_MAGIC_V2 {
            let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
            let header = [u32_at(10), u32_at(14), u32_at(18), u32_at(22), u32_at(26), u32_at(30)];
            let want = [
                NNUE_VERSION_V2,
                NNUE_FEATURES_V2 as u32,
                NNUE_HIDDEN as u32,
                NNUE_BUCKETS as u32,
                QA as u32,
                QB as u32,
            ];
            if header != want {
                return Err(format!("SANTNNUE02 header {header:?} unsupported (expected {want:?})"));
            }
            NNUE_FEATURES_V2
        } else {
            return Err("not a SANTNNUE03/SANTNNUE02 file (bad magic; run scripts/convert_checkpoints.py)".into());
        };
        let expected = HEADER_BYTES
            + file_features * NNUE_HIDDEN * 2
            + NNUE_HIDDEN * 2
            + NNUE_BUCKETS * 2 * NNUE_HIDDEN * 2
            + NNUE_BUCKETS * 4;
        if bytes.len() != expected {
            return Err(format!("NNUE size mismatch: got {} bytes, expected {expected}", bytes.len()));
        }
        let mut o = HEADER_BYTES;
        let mut i16_at = || {
            let v = i16::from_le_bytes([bytes[o], bytes[o + 1]]);
            o += 2;
            v
        };
        let mut n = Self::zeros();
        n.ft_w[..file_features * NNUE_HIDDEN].iter_mut().for_each(|w| *w = i16_at());
        n.ft_b.iter_mut().for_each(|w| *w = i16_at());
        n.out_w.iter_mut().flatten().for_each(|w| *w = i16_at());
        let mut o = expected - NNUE_BUCKETS * 4;
        for w in &mut n.out_b {
            *w = i32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
            o += 4;
        }
        Ok(n)
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        Self::from_bytes(&bytes)
    }

    #[inline(always)]
    fn add_row(&self, acc: &mut [i16; NNUE_HIDDEN], f: u16) {
        let row = &self.ft_w[f as usize * NNUE_HIDDEN..(f as usize + 1) * NNUE_HIDDEN];
        for (a, &w) in acc.iter_mut().zip(row) {
            *a = a.wrapping_add(w);
        }
    }

    #[inline(always)]
    fn sub_row(&self, acc: &mut [i16; NNUE_HIDDEN], f: u16) {
        let row = &self.ft_w[f as usize * NNUE_HIDDEN..(f as usize + 1) * NNUE_HIDDEN];
        for (a, &w) in acc.iter_mut().zip(row) {
            *a = a.wrapping_sub(w);
        }
    }

    /// Brings `cache` slot `p` up to date with `feats`.
    #[inline]
    fn update(&self, cache: &mut NnueAccCache, p: usize, feats: &FeatureList) {
        if !cache.valid[p] {
            cache.acc[p] = self.ft_b;
            for &f in feats.as_slice() {
                self.add_row(&mut cache.acc[p], f);
            }
        } else {
            let old = cache.feats[p];
            let (o, n) = (old.as_slice(), feats.as_slice());
            let acc = &mut cache.acc[p];
            let common = o.len().min(n.len());
            for k in 0..common {
                if o[k] != n[k] {
                    self.sub_row(acc, o[k]);
                    self.add_row(acc, n[k]);
                }
            }
            for &f in &o[common..] {
                self.sub_row(acc, f);
            }
            for &f in &n[common..] {
                self.add_row(acc, f);
            }
        }
        cache.feats[p] = *feats;
        cache.valid[p] = true;
    }

    /// Side-to-move logit (pre-tanh). Uses and refreshes `cache`.
    #[inline]
    pub fn evaluate(&self, s: &CompactSantoriniState, cache: &mut NnueAccCache) -> f32 {
        let mut f = [FeatureList::default(); 2];
        write_features_both(s, &mut f);
        self.update(cache, 0, &f[0]);
        self.update(cache, 1, &f[1]);
        let stm = s.current_player as usize;
        let w = &self.out_w[bucket(s)];
        let mut sum = 0i32;
        for (k, persp) in [stm, 1 - stm].into_iter().enumerate() {
            let acc = &cache.acc[persp];
            let wk = &w[k * NNUE_HIDDEN..(k + 1) * NNUE_HIDDEN];
            // i16 clamp then widening multiply-add: the shape LLVM lowers to vpmaddwd.
            for (&a, &wi) in acc.iter().zip(wk) {
                sum += a.clamp(0, QA as i16) as i32 * wi as i32;
            }
        }
        (sum + self.out_b[bucket(s)]) as f32 / (QA * QB) as f32
    }
}
