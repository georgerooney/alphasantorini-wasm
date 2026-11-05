// AlphaSantorini — In-browser WebAssembly NNUE engine & arena client (`SANTNNUE03`, 44 powers).
// Vanilla JS; all DOM/SVG constructed via createElement(NS) and textContent (zero innerHTML).
"use strict";

const SVG_NS = "http://www.w3.org/2000/svg";
const CELL = 100;
const $ = (id) => document.getElementById(id);

// Concise mathematical mechanics summaries across all 44 supported powers (no verbatim card prose).
const GOD_LIST = [
  // Standard & intro (11)
  { id: 0, intro: true, name: "Mortal", desc: "Standard rules: move 1 adjacent step (up at most +1 level), then build 1 adjacent block or level-3 dome." },
  { id: 1, intro: true, name: "Apollo", desc: "Move: may step onto an adjacent opponent on the same or lower level, swapping places." },
  { id: 2, intro: true, name: "Artemis", desc: "Move: may take 1 additional step before building, never returning to the starting square." },
  { id: 3, intro: true, name: "Athena", desc: "Passive: if your worker moved up on your turn, opponent workers cannot move up on their next turn." },
  { id: 4, intro: true, name: "Atlas", desc: "Build: may place a dome on any level (0, 1, 2, or 3)." },
  { id: 5, intro: true, name: "Demeter", desc: "Build: may build 1 additional time on a different adjacent square." },
  { id: 6, intro: true, name: "Hephaestus", desc: "Build: may add 1 extra non-dome block on the same square just built." },
  { id: 7, intro: true, name: "Hermes", desc: "Turn: if you neither climb nor drop, either or both workers may walk any number of level steps (including 0), then one builds." },
  { id: 8, intro: true, name: "Minotaur", desc: "Move: may step onto an adjacent opponent if the next square in line is open, pushing them onto it." },
  { id: 9, intro: true, name: "Pan", desc: "Win condition: also wins immediately by moving down 2 or more levels." },
  { id: 10, intro: true, name: "Prometheus", desc: "Turn: if your worker does not move up, it may build both before and after moving." },
  // Pantheon (33)
  { id: 11, intro: false, name: "Aphrodite", desc: "Constraint: an opponent worker starting its turn adjacent to yours must end its turn adjacent to one of yours." },
  { id: 12, intro: false, name: "Ares", desc: "End of turn: may remove an unoccupied non-dome block adjacent to your unmoved worker." },
  { id: 13, intro: false, name: "Bia", desc: "Setup & move: places workers first; moving in line behind an adjacent opponent removes that opponent worker." },
  { id: 15, intro: false, name: "Charon", desc: "Before move: may flip an adjacent opponent worker to the open square directly behind your worker." },
  { id: 16, intro: false, name: "Chronus", desc: "Win condition: also wins when at least 4 complete 4-piece towers stand on the board at turn end." },
  { id: 19, intro: false, name: "Eros", desc: "Setup & win: places workers on opposite board edges; also wins at turn end if both workers stand adjacent on level 1." },
  { id: 20, intro: false, name: "Hera", desc: "Constraint: opponent workers cannot win by stepping onto any of the 16 perimeter squares." },
  { id: 21, intro: false, name: "Hestia", desc: "Build: may build 1 additional time on any adjacent interior (non-perimeter) square." },
  { id: 22, intro: false, name: "Hypnus", desc: "Constraint: at the start of an opponent turn, if one of their workers is strictly higher, that worker cannot move." },
  { id: 23, intro: false, name: "Limus", desc: "Constraint: opponents cannot build blocks on squares adjacent to your workers (building level-3 domes remains legal)." },
  { id: 24, intro: false, name: "Medusa", desc: "End of turn: may replace lower-level opponent workers adjacent to yours with domes." },
  { id: 26, intro: false, name: "Persephone", desc: "Constraint: if an opponent has any legal turn that climbs up a level, they must climb." },
  { id: 27, intro: false, name: "Poseidon", desc: "End of turn: if your unmoved worker stands on ground level (0), it may build up to 2 extra times." },
  { id: 28, intro: false, name: "Selene", desc: "Build: instead of a normal build, your female (\u2640, first-placed) worker may place a dome at any level adjacent to her." },
  { id: 29, intro: false, name: "Triton", desc: "Move: each time your moving worker steps onto a perimeter square, it may immediately move again." },
  { id: 30, intro: false, name: "Zeus", desc: "Build: may build a block directly underneath the moved worker instead of on an adjacent square (cannot win by doing so)." },
  { id: 37, intro: false, name: "Hades", desc: "Constraint: opponent workers cannot move down to a lower level." },
  { id: 38, intro: false, name: "Harpies", desc: "Constraint: each opponent step slides forward in the same direction until blocked by height, dome, worker, or board edge." },
  { id: 44, intro: false, name: "Terpsichore", desc: "Turn: both of your workers must move, and then both of your workers must build." },
  { id: 45, intro: false, name: "Urania", desc: "Move & build: board edges wrap toroidally for both your moves and your builds." },
  { id: 46, intro: false, name: "Achilles", desc: "Hero (once): build both before and after moving with the same worker (climbing allowed)." },
  { id: 48, intro: false, name: "Atalanta", desc: "Hero (once): your moving worker may take any number of extra steps before building." },
  { id: 49, intro: false, name: "Bellerophon", desc: "Hero (once): your worker may climb up 2 levels in a single step (and win onto level 3)." },
  { id: 50, intro: false, name: "Heracles", desc: "Hero (once): instead of building normally, both workers may place any number of domes on adjacent squares." },
  { id: 54, intro: false, name: "Polyphemus", desc: "Hero (once, end of turn): place up to 2 domes on any unoccupied squares on the board." },
  { id: 55, intro: false, name: "Theseus", desc: "Hero (once, end of turn): remove any adjacent opponent workers standing at least 2 levels above yours." },
  { id: 57, intro: false, name: "Scylla", desc: "Move: after stepping, may pull an opponent worker adjacent to the vacated square onto that vacated square." },
  { id: 58, intro: false, name: "Castor & Pollux", desc: "Turn: instead of normal play, either move both workers without building, or build with both workers without moving." },
  { id: 61, intro: false, name: "Maenads", desc: "Win condition: also wins at turn end if your two workers flank an opponent worker on opposite adjacent squares." },
  { id: 62, intro: false, name: "Asteria", desc: "End of turn: if your worker moved down this turn, you may place a dome on any unoccupied square." },
  { id: 63, intro: false, name: "Hippolyta", desc: "Constraint: your female (\u2640, first-placed) worker moves in all 8 directions; all other workers move diagonally only." },
  { id: 65, intro: false, name: "Iris", desc: "Move: may leap straight over an adjacent worker onto an unoccupied square at any height." },
  { id: 67, intro: false, name: "Pegasus", desc: "Move: may climb 2 or more levels in one step, but cannot win on a climb of 2+ levels." },
];

const GOD_BY_ID = new Map(GOD_LIST.map((g) => [g.id, g]));
const godInfo = (id) => GOD_BY_ID.get(id) || { id, name: `God ${id}`, desc: "" };

let worker = null;
let msgId = 0;
const pending = new Map();

function sendWorker(type, payload = {}) {
  return new Promise((resolve, reject) => {
    const id = ++msgId;
    pending.set(id, { resolve, reject });
    worker.postMessage({ id, type, payload });
  });
}

let view = null;
// `gameMode`: "p0" (human plays First/Navy), "p1" (human plays Second/Ochre), or "bots" (Bot vs. Bot).
let gameMode = "p0";
let botsPaused = false;
// Monotonic epoch token so switching modes, starting a new game, or undoing cleanly cancels any in-flight bot loop.
let runEpoch = 0;
let busy = false;
let sel = { from: null, to: null };
let lastSubTurnWorkerSq = null;
// Why `actionIntent`: seven powers (Prometheus, Achilles, Hermes, Triton, Atalanta, Castor & Pollux,
// and Artemis in STAGE_ARTEMIS) can legally either move onto OR build-only on the same empty square.
let actionIntent = "move"; // "move" | "build"
let domeChoice = null;
let hintAction = null;
let activePvRank = null;
let lastAnalysis = null;

const sqName = (sq) => (sq !== undefined && sq !== null && sq < 25 ? "abcde"[sq % 5] + (5 - Math.floor(sq / 5)) : "?");
const activeHumanSide = () => (gameMode === "p0" ? 0 : gameMode === "p1" ? 1 : -1);
const isBotSeat = (p) => gameMode === "bots" || p !== activeHumanSide();
const seatColor = (p) => (p === 0 ? "Navy" : "Ochre");
const seatBudgetMs = (p) => {
  const el = $(p === 0 ? "p0-ms" : "p1-ms");
  const raw = el ? parseInt(el.value, 10) : 150;
  return Math.max(15, Math.min(5000, Number.isFinite(raw) ? raw : 150));
};

function syncSeatLabels() {
  for (const p of [0, 1]) {
    const bot = isBotSeat(p);
    const roleEl = $(p === 0 ? "p0-role" : "p1-role");
    const msLabelEl = $(p === 0 ? "p0-ms-label" : "p1-ms-label");
    if (roleEl) roleEl.textContent = bot ? "Bot" : "You";
    if (msLabelEl) msLabelEl.textContent = bot ? "Bot search (ms)" : "Hint / analyze (ms)";
  }
  const pauseBtn = $("pause-bots");
  const stepBtn = $("step-bot");
  const inBots = gameMode === "bots";
  if (pauseBtn) {
    pauseBtn.classList.toggle("hidden", !inBots);
    pauseBtn.textContent = botsPaused ? "Resume" : "Pause";
    pauseBtn.disabled = Boolean(view && view.game_over);
  }
  if (stepBtn) {
    stepBtn.classList.toggle("hidden", !inBots);
    stepBtn.disabled = busy || !botsPaused || !view || Boolean(view.game_over);
  }
}

function resetSel() {
  sel = { from: null, to: null };
  lastSubTurnWorkerSq = null;
  actionIntent = "move";
  domeChoice = null;
}

function humanToMove() {
  const hSide = activeHumanSide();
  return Boolean(view && !view.game_over && hSide !== -1 && view.current_player === hSide);
}

function svg(tag, attrs, parent) {
  const el = document.createElementNS(SVG_NS, tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  if (parent) parent.appendChild(el);
  return el;
}

function showError(msg) {
  const el = $("error");
  el.textContent = msg || "";
  el.classList.toggle("hidden", !msg);
}

function showHexStatus(msg) {
  const el = $("hex-status");
  if (el) el.textContent = msg || "";
}

function fmtScore(v) {
  if (v === null || v === undefined || Number.isNaN(v)) return "—";
  const sign = v >= 0 ? "+" : "\u2212";
  return sign + Math.abs(v).toFixed(2);
}

function legalFor(pred) {
  return view && view.legal_actions ? view.legal_actions.filter(pred) : [];
}

function movableWorkers() {
  if (!view || !humanToMove() || view.setup || busy) return new Set();
  return new Set(view.legal_actions.filter((a) => a.from !== undefined).map((a) => a.from));
}

function workerHasMoveAndBuild(fromSq) {
  if (fromSq === null) return false;
  const hasMove = legalFor((a) => a.from === fromSq && (a.kind === "turn" || a.kind === "move" || a.kind === "zeus")).length > 0;
  const hasBuild = legalFor((a) => a.from === fromSq && a.kind === "build").length > 0;
  return hasMove && hasBuild;
}

function moveOnlyButtonLabel(godId) {
  switch (godId) {
    case 2: return "Move again (Artemis)";
    case 7: return "Keep walking (Hermes)";
    case 29: return "Chain move (Triton)";
    case 48: return "Sprint again (spend Hero)";
    case 50: return "Build domes (spend Hero)";
    case 57: return "Drag neighbor (Scylla)";
    case 58: return "Move both workers";
    default: return "Move only (use power)";
  }
}

function viewFemaleSq(v) {
  if (!v || !v.female) return [null, null];
  return [0, 1].map((p) => {
    const idx = (v.female[p] || []).findIndex(Boolean);
    return idx >= 0 && v.workers[p][idx] !== 255 ? v.workers[p][idx] : null;
  });
}

function targets() {
  if (!view || !humanToMove() || busy) return [];
  if (view.setup) {
    return legalFor((a) => a.kind === "place").map((a) => ({ sq: a.square, kind: "place", action: a.action }));
  }
  const squares = legalFor((a) => a.kind === "square").map((a) => ({ sq: a.square, kind: "square", action: a.action }));
  if (squares.length) return squares;

  // Heracles (`STAGE_HERACLES`): only `dome` + `pass` exist across both workers; highlight every
  // legal adjacent dome square from both workers at once so no manual worker switch is needed.
  const onlyDomesAndPass =
    view.in_sub_turn &&
    view.legal_actions.length > 0 &&
    view.legal_actions.every((a) => a.kind === "dome" || a.kind === "pass");
  if (onlyDomesAndPass) {
    const seen = new Map();
    for (const a of legalFor((a) => a.kind === "dome")) {
      if (!seen.has(a.build)) seen.set(a.build, { sq: a.build, kind: "domeonly", action: a.action });
    }
    return [...seen.values()];
  }

  if (sel.from === null) return [];

  if (sel.to === null) {
    const seen = new Map();
    if (workerHasMoveAndBuild(sel.from) && actionIntent === "build") {
      for (const a of legalFor((a) => a.from === sel.from && a.kind === "build")) {
        seen.set(a.build, { sq: a.build, kind: "buildonly", action: a.action });
      }
      return [...seen.values()];
    }
    for (const a of legalFor((a) => a.from === sel.from && (a.kind === "turn" || a.kind === "move"))) {
      const prev = seen.get(a.to);
      if (!prev || a.win) seen.set(a.to, { sq: a.to, kind: a.win ? "win" : "move", action: a.win ? a.action : null });
    }
    for (const a of legalFor((a) => a.from === sel.from && a.kind === "zeus")) {
      if (!seen.has(a.to)) seen.set(a.to, { sq: a.to, kind: "move", action: null });
    }
    for (const a of legalFor((a) => a.from === sel.from && a.kind === "build")) {
      if (!seen.has(a.build)) seen.set(a.build, { sq: a.build, kind: "buildonly", action: a.action });
    }
    for (const a of legalFor((a) => a.from === sel.from && a.kind === "dome")) {
      seen.set(a.build, { sq: a.build, kind: "domeonly", action: a.action });
    }
    for (const a of legalFor((a) => a.from === sel.from && a.kind === "force")) {
      seen.set(a.forced_from, { sq: a.forced_from, kind: "force", action: a.action });
    }
    return [...seen.values()];
  }

  // Build phase after selecting `(sel.from, sel.to)`
  const seen = new Map();
  for (const a of legalFor((a) => a.kind === "turn" && a.from === sel.from && a.to === sel.to && !a.win)) {
    if (!seen.has(a.build)) seen.set(a.build, { sq: a.build, kind: "build", action: null });
  }
  for (const a of legalFor((a) => a.kind === "zeus" && a.from === sel.from && a.to === sel.to)) {
    seen.set(a.to, { sq: a.to, kind: "buildonly", action: a.action });
  }
  return [...seen.values()];
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

function renderBoard() {
  const boardEl = $("board");
  boardEl.replaceChildren();
  if (!view) return;

  const movable = movableWorkers();
  const onlyDomes =
    view.in_sub_turn &&
    view.legal_actions.length > 0 &&
    view.legal_actions.every((a) => a.kind === "dome" || a.kind === "pass");
  if (sel.from === null && view.in_sub_turn && !onlyDomes && movable.size >= 1) {
    sel.from = lastSubTurnWorkerSq !== null && movable.has(lastSubTurnWorkerSq)
      ? lastSubTurnWorkerSq
      : [...movable][0];
  }

  const tgs = targets();
  const tgMap = new Map(tgs.map((t) => [t.sq, t]));

  const hSide = activeHumanSide();
  const displayWorkers = [[...view.workers[0]], [...view.workers[1]]];
  const femSq = viewFemaleSq(view);
  const inBuildPhase = humanToMove() && sel.from !== null && sel.to !== null;
  if (inBuildPhase) {
    const idx = displayWorkers[hSide].indexOf(sel.from);
    if (idx !== -1) {
      if (femSq[hSide] === sel.from) femSq[hSide] = sel.to;
      displayWorkers[hSide][idx] = sel.to;
    }
    // Preview Apollo swap, Minotaur push, or Bia elimination on the opponent worker
    const sample = legalFor((a) => (a.kind === "turn" || a.kind === "zeus") && a.from === sel.from && a.to === sel.to)[0];
    if (sample && sample.forced_from !== undefined) {
      const opp = 1 - hSide;
      const oIdx = displayWorkers[opp].indexOf(sample.forced_from);
      if (oIdx !== -1) {
        const dst = sample.forced_to !== undefined && sample.forced_to !== null ? sample.forced_to : 255;
        if (femSq[opp] === sample.forced_from) femSq[opp] = dst !== 255 ? dst : null;
        displayWorkers[opp][oIdx] = dst;
      }
    }
  }

  const occupiedPreview = new Set([...displayWorkers[0], ...displayWorkers[1]].filter((w) => w !== 255));
  const lowDomes = view.low_domes || 0;

  for (let sq = 0; sq < 25; sq++) {
    const r = Math.floor(sq / 5);
    const c = sq % 5;
    const x = c * CELL;
    const y = r * CELL;
    const h = view.height[sq];

    svg("rect", {
      x: x + 2,
      y: y + 2,
      width: CELL - 4,
      height: CELL - 4,
      rx: 8,
      class: "sq-ground",
      "data-sq": sq,
    }, boardEl);

    const coord = svg("text", { x: x + 8, y: y + 16, class: "coord-label" }, boardEl);
    coord.textContent = sqName(sq);

    if (h >= 1 && (h < 4 || ((lowDomes >> sq) & 1) === 0)) {
      svg("rect", { x: x + 10, y: y + 10, width: CELL - 20, height: CELL - 20, rx: 6, class: "block-l1" }, boardEl);
    }
    if (h >= 2 && (h < 4 || ((lowDomes >> sq) & 1) === 0)) {
      svg("rect", { x: x + 18, y: y + 18, width: CELL - 36, height: CELL - 36, rx: 5, class: "block-l2" }, boardEl);
    }
    if (h >= 3 && (h < 4 || ((lowDomes >> sq) & 1) === 0)) {
      svg("rect", { x: x + 26, y: y + 26, width: CELL - 52, height: CELL - 52, rx: 4, class: "block-l3" }, boardEl);
    }
    if (h === 4) {
      svg("circle", { cx: x + CELL / 2, cy: y + CELL / 2, r: 21, class: "dome" }, boardEl);
      if ((lowDomes >> sq) & 1) {
        svg("circle", { cx: x + CELL / 2, cy: y + CELL / 2, r: 12, class: "low-dome-ring" }, boardEl);
      }
    }

    if (inBuildPhase && sq === sel.from && !occupiedPreview.has(sq)) {
      svg("circle", {
        cx: x + CELL / 2,
        cy: y + CELL / 2,
        r: 19,
        class: "worker-ghost",
        stroke: hSide === 0 ? "var(--p0-color)" : "var(--p1-color)",
      }, boardEl);
    }

    for (let p = 0; p < 2; p++) {
      for (const wsq of displayWorkers[p]) {
        if (wsq === sq && wsq !== 255) {
          const isSelected = sel.from === sq && !inBuildPhase;
          const isPreview = inBuildPhase && sq === sel.to && p === hSide;
          if (isSelected || isPreview) {
            svg("circle", { cx: x + CELL / 2, cy: y + CELL / 2, r: 25, class: "selected-ring" }, boardEl);
          }
          const selectable = !inBuildPhase && p === hSide && movable.has(sq);
          const cls = `worker-p${p}${isPreview ? " worker-preview" : ""}${selectable ? " selectable" : ""}`;
          svg("circle", { cx: x + CELL / 2, cy: y + CELL / 2, r: 19, class: cls, "data-sq": sq }, boardEl);

          if (femSq[p] === sq) {
            const fg = svg("text", { x: x + CELL / 2, y: y + CELL / 2, class: "female-glyph" }, boardEl);
            fg.textContent = "\u2640";
          } else {
            svg("circle", {
              cx: x + CELL / 2,
              cy: y + CELL / 2,
              r: 5.5,
              fill: p === 0 ? "#ffffff" : "#fef3c7",
              "pointer-events": "none",
            }, boardEl);
          }
        }
      }
    }

    if (tgMap.has(sq)) {
      const tg = tgMap.get(sq);
      const isDomeChoiceSq = domeChoice && domeChoice.sq === sq;
      let ringClass = "target-ring";
      if (isDomeChoiceSq) ringClass = "target-ring dome-choice";
      else if (tg.kind === "win") ringClass = "target-ring win";
      else if (tg.kind === "build" || tg.kind === "buildonly" || tg.kind === "domeonly") ringClass = "target-ring build";
      else if (tg.kind === "square" || tg.kind === "force") ringClass = "target-ring special";

      svg("circle", {
        cx: x + CELL / 2,
        cy: y + CELL / 2,
        r: 23,
        class: ringClass,
        "data-target-sq": sq,
      }, boardEl);
    }

    if (
      hintAction &&
      [
        hintAction.square,
        hintAction.from,
        hintAction.to,
        hintAction.build,
        hintAction.forced_from,
        hintAction.forced_to,
        hintAction.removed_block,
      ].includes(sq)
    ) {
      svg("rect", {
        x: x + 5,
        y: y + 5,
        width: CELL - 10,
        height: CELL - 10,
        rx: 6,
        fill: "none",
        stroke: "#166534",
        "stroke-width": 2.2,
        "stroke-dasharray": "5 3",
        "pointer-events": "none",
      }, boardEl);
    }
  }

  updateChrome();
}

function makeBtn(label, cls, onClick) {
  const b = document.createElement("button");
  b.type = "button";
  b.className = cls || "btn";
  b.textContent = label;
  b.onclick = onClick;
  return b;
}

function renderAnalysisPanel() {
  const evalEl = $("an-eval");
  evalEl.classList.remove("pos", "neg");
  const pvTable = $("pv-table");
  const pvBody = $("pv-body");
  pvBody.replaceChildren();

  if (!lastAnalysis || !view) {
    pvTable.classList.add("hidden");
    return;
  }

  const inBots = gameMode === "bots";
  const refSide = inBots ? 0 : activeHumanSide();
  const persp = lastAnalysis.perspective !== undefined ? lastAnalysis.perspective : view.current_player;
  const sign = persp === refSide ? 1 : -1;
  const v = sign * lastAnalysis.value;
  if (lastAnalysis.mate_in_plies !== undefined && lastAnalysis.mate_in_plies !== null) {
    const who = inBots ? (v >= 0 ? "Navy" : "Ochre") : (v >= 0 ? "You" : "Bot");
    evalEl.textContent = `${who} mate in ${Math.ceil(lastAnalysis.mate_in_plies / 2)}`;
  } else {
    evalEl.textContent = inBots ? `${fmtScore(v)} (Navy)` : `${fmtScore(v)} for you`;
  }
  evalEl.classList.add(v >= 0 ? "pos" : "neg");

  $("an-depth").textContent = lastAnalysis.depth ? `depth ${lastAnalysis.depth}` : "—";
  $("an-nodes").textContent = lastAnalysis.nodes ? lastAnalysis.nodes.toLocaleString() : "—";
  $("an-ms").textContent = lastAnalysis.elapsed_ms !== undefined ? `${lastAnalysis.elapsed_ms.toFixed(0)} ms` : "—";
  if (lastAnalysis.steps && lastAnalysis.steps.length) {
    $("an-pv").textContent = lastAnalysis.pv && lastAnalysis.pv.length
      ? lastAnalysis.pv.join(" ")
      : lastAnalysis.steps.map(formatStep).join(", ");
  }

  const lines = Array.isArray(lastAnalysis.lines) ? lastAnalysis.lines : [];
  pvTable.classList.toggle("hidden", lines.length === 0);
  for (const ln of lines) {
    const tr = document.createElement("tr");
    if (activePvRank === ln.rank) tr.classList.add("active-pv");

    const tdRank = document.createElement("td");
    tdRank.className = "mono";
    tdRank.textContent = String(ln.rank);

    const tdMove = document.createElement("td");
    tdMove.className = "mono";
    tdMove.textContent = ln.short || formatStep(ln.move_desc);

    const tdEval = document.createElement("td");
    const lv = sign * ln.value;
    tdEval.className = `mono ${lv >= 0 ? "pos" : "neg"}`;
    if (ln.mate_in_plies !== undefined && ln.mate_in_plies !== null) {
      tdEval.textContent = `${lv >= 0 ? "+M" : "\u2212M"}${Math.ceil(ln.mate_in_plies / 2)}`;
    } else {
      tdEval.textContent = fmtScore(lv);
    }

    const tdPv = document.createElement("td");
    tdPv.className = "mono";
    tdPv.textContent = Array.isArray(ln.pv) && ln.pv.length ? ln.pv.join(" ") : (ln.short || "—");

    tr.appendChild(tdRank);
    tr.appendChild(tdMove);
    tr.appendChild(tdEval);
    tr.appendChild(tdPv);
    tr.onclick = () => {
      activePvRank = ln.rank;
      hintAction = ln.move_desc;
      renderBoard();
    };
    pvBody.appendChild(tr);
  }
}

function updateChrome() {
  const statusEl = $("status");
  const turnEl = $("turn-info");
  const instrEl = $("instructions");
  const actBar = $("action-bar");
  const godsLine = $("gods-line");
  const godsRules = $("gods-rules");

  actBar.replaceChildren();
  godsLine.replaceChildren();
  godsRules.replaceChildren();
  syncSeatLabels();

  if (!view) return;

  const hSide = activeHumanSide();
  const hexInput = $("state-hex");
  if (hexInput && view.state_hex && document.activeElement !== hexInput) {
    hexInput.value = view.state_hex;
  }

  for (const p of [0, 1]) {
    const g = godInfo(view.gods[p]);
    const color = seatColor(p);
    const role = isBotSeat(p) ? `Bot (${seatBudgetMs(p)} ms)` : "You";
    const badge = document.createElement("span");
    badge.className = `god-badge p${p}`;
    badge.textContent = `${color} · ${role}: ${g.name}`;
    badge.title = g.desc;
    godsLine.appendChild(badge);

    if (view.power_used && view.power_used[p]) {
      const chip = document.createElement("span");
      chip.className = "chip";
      chip.textContent = `${color} Hero spent`;
      godsLine.appendChild(chip);
    }
    if (view.restricted && view.restricted[p]) {
      const rChip = document.createElement("span");
      rChip.className = "chip crimson";
      rChip.textContent = `${color}: no moving up (Athena)`;
      godsLine.appendChild(rChip);
    }

    const ruleLine = document.createElement("div");
    ruleLine.textContent = `${g.name}: ${g.desc}`;
    godsRules.appendChild(ruleLine);
  }

  turnEl.textContent = gameMode === "bots"
    ? `Ply ${view.turn_count} · Bot vs. Bot`
    : `Ply ${view.turn_count} · you play ${hSide === 0 ? "Navy (1st)" : "Ochre (2nd)"}`;
  renderAnalysisPanel();

  if (view.game_over) {
    if (gameMode === "bots") {
      const w = view.winner;
      statusEl.textContent = w === 0 || w === 1
        ? `Game over — ${seatColor(w)} (${godInfo(view.gods[w]).name}) wins`
        : "Game over";
    } else {
      const won = view.winner === hSide;
      statusEl.textContent = won ? "Game over — you win" : `Game over — ${seatColor(1 - hSide)} bot wins`;
    }
    instrEl.textContent = "Start a new game, undo, or copy the position hex.";
    $("undo").disabled = busy || !view.can_undo;
    $("hint").disabled = true;
    $("analyze").disabled = true;
    return;
  }

  $("undo").disabled = busy || !view.can_undo;
  $("hint").disabled = busy || !humanToMove();
  $("analyze").disabled = busy;

  if (gameMode === "bots") {
    const mover = view.current_player;
    if (botsPaused) {
      statusEl.textContent = `Paused — ${seatColor(mover)} to move`;
      instrEl.textContent = "Click Step to advance one bot turn, Analyze to inspect lines, or Resume.";
    } else {
      statusEl.textContent = `Bot thinking (${seatColor(mover)} · ${seatBudgetMs(mover)} ms)…`;
      instrEl.textContent = "Bots playing — click Pause at any time to step or analyze.";
    }
    return;
  }

  if (!humanToMove()) {
    const mover = view.current_player;
    statusEl.textContent = `Bot thinking (${seatColor(mover)} · ${seatBudgetMs(mover)} ms)…`;
    instrEl.textContent = "Searching alpha-beta variations…";
    return;
  }

  statusEl.textContent = view.in_sub_turn
    ? `Your move — continued (${seatColor(hSide)})`
    : `Your move (${seatColor(hSide)})`;

  if (view.setup) {
    instrEl.textContent = "Setup: click an empty highlighted square to place a worker.";
    return;
  }

  if (domeChoice) {
    const targetHeight = view.height[domeChoice.sq];
    const dName = godInfo(view.gods[hSide]).name;
    instrEl.textContent = `${dName}: build a block (level ${targetHeight + 1}) or a dome on ${sqName(domeChoice.sq)}.`;
    actBar.appendChild(makeBtn(`Build block (level ${targetHeight + 1})`, "btn primary", () => {
      const act = domeChoice.block;
      domeChoice = null;
      doAction(act);
    }));
    actBar.appendChild(makeBtn("Build dome", "btn terra", () => {
      const act = domeChoice.dome;
      domeChoice = null;
      doAction(act);
    }));
    actBar.appendChild(makeBtn("Cancel", "btn", () => {
      domeChoice = null;
      renderBoard();
    }));
    return;
  }

  const passAction = view.legal_actions.find((a) => a.kind === "pass");
  if (passAction) {
    actBar.appendChild(makeBtn("End turn (skip optional step)", "btn accent", () => doAction(passAction.action)));
  }

  if (sel.from !== null && sel.to === null && workerHasMoveAndBuild(sel.from)) {
    const label = actionIntent === "build" ? "Move instead" : "Build instead (use power)";
    actBar.appendChild(makeBtn(label, actionIntent === "build" ? "btn" : "btn primary", () => {
      actionIntent = actionIntent === "build" ? "move" : "build";
      renderBoard();
    }));
  }

  if (sel.from !== null && sel.to !== null) {
    const moveOnly = view.legal_actions.find((a) => a.kind === "move" && a.from === sel.from && a.to === sel.to);
    if (moveOnly) {
      actBar.appendChild(makeBtn(moveOnlyButtonLabel(view.gods[hSide]), "btn primary", () => doAction(moveOnly.action)));
    }
    const zeusSelf = view.legal_actions.find((a) => a.kind === "zeus" && a.from === sel.from && a.to === sel.to);
    if (zeusSelf) {
      actBar.appendChild(makeBtn("Build under worker (Zeus)", "btn terra", () => doAction(zeusSelf.action)));
    }
    actBar.appendChild(makeBtn(`Change move (from ${sqName(sel.from)})`, "btn", () => {
      sel.to = null;
      domeChoice = null;
      renderBoard();
    }));
  }

  const squareTargets = legalFor((a) => a.kind === "square");
  const onlyDomeTargets =
    view.in_sub_turn &&
    view.legal_actions.length > 0 &&
    view.legal_actions.every((a) => a.kind === "dome" || a.kind === "pass");
  if (squareTargets.length > 0 || onlyDomeTargets) {
    instrEl.textContent = "Select a highlighted square to apply your power, or end turn.";
  } else if (sel.from === null) {
    instrEl.textContent = "Select one of your workers.";
  } else if (sel.to === null) {
    instrEl.textContent = actionIntent === "build"
      ? `Worker at ${sqName(sel.from)}: click an orange target square to build.`
      : `Worker at ${sqName(sel.from)}: click a highlighted square to move or act.`;
  } else {
    instrEl.textContent = `Moving to ${sqName(sel.to)}: click an orange square to build.`;
  }
}

// ---------------------------------------------------------------------------
// Board Interaction
// ---------------------------------------------------------------------------

function onSquareClick(sq) {
  if (!view || !humanToMove() || busy) return;
  const hSide = activeHumanSide();

  if (view.setup) {
    const a = view.legal_actions.find((act) => act.kind === "place" && act.square === sq);
    if (a) doAction(a.action);
    return;
  }

  const tgs = targets();
  const target = tgs.find((t) => t.sq === sq);

  // Build phase: worker & destination already chosen
  if (sel.from !== null && sel.to !== null) {
    if (target && target.kind === "buildonly" && target.action !== null) {
      // Zeus self-build on `sel.to` must fire BEFORE `sq === sel.to` cancel!
      doAction(target.action);
      return;
    }
    if (target && target.kind === "build") {
      const turns = legalFor((a) => a.kind === "turn" && a.from === sel.from && a.to === sel.to && a.build === sq);
      const blockAct = turns.find((a) => !a.dome);
      const domeAct = turns.find((a) => a.dome);
      if (blockAct && domeAct) {
        domeChoice = { sq, block: blockAct.action, dome: domeAct.action };
        renderBoard();
        return;
      }
      const chosen = blockAct || domeAct || turns[0];
      if (chosen) {
        doAction(chosen.action);
        return;
      }
    }
    if (sq === sel.to) {
      sel.to = null;
      domeChoice = null;
      renderBoard();
      return;
    }
    const movables = movableWorkers();
    if (movables.has(sq) && sq !== sel.from) {
      sel = { from: sq, to: null };
      actionIntent = "move";
      domeChoice = null;
      renderBoard();
      return;
    }
    return;
  }

  // Square-target powers (Ares, Medusa, Theseus, Polyphemus, Asteria, Scylla)
  if (target && target.kind === "square") {
    doAction(target.action);
    return;
  }

  // Worker selection
  const isOwnWorker = view.workers[hSide].includes(sq);
  const movables = movableWorkers();
  if (isOwnWorker && movables.has(sq)) {
    sel = sel.from === sq ? { from: null, to: null } : { from: sq, to: null };
    actionIntent = "move";
    domeChoice = null;
    renderBoard();
    return;
  }

  if (!target) {
    if (sel.from !== null && !view.in_sub_turn) {
      resetSel();
      renderBoard();
    }
    return;
  }

  if (target.kind === "win") {
    doAction(target.action);
    return;
  }

  if (target.kind === "move") {
    const turns = legalFor((a) => (a.kind === "turn" || a.kind === "zeus") && a.from === sel.from && a.to === sq);
    const moves = legalFor((a) => a.kind === "move" && a.from === sel.from && a.to === sq);
    if (!turns.length && moves.length) {
      doAction(moves[0].action);
      return;
    }
    sel.to = sq;
    renderBoard();
    return;
  }

  if (target.action !== null && target.action !== undefined) {
    doAction(target.action);
  }
}

// ---------------------------------------------------------------------------
// Game Engine Communication
// ---------------------------------------------------------------------------

function formatStep(s) {
  if (!s) return "—";
  switch (s.kind) {
    case "place":
      return `place ${sqName(s.square)}`;
    case "turn": {
      const forced = s.forced_to !== undefined && s.forced_to !== null
        ? ` (push→${sqName(s.forced_to)})`
        : s.forced_from !== undefined
          ? ` (remove ${sqName(s.forced_from)})`
          : "";
      if (s.win) return `${sqName(s.from)}→${sqName(s.to)}${forced} wins`;
      return `${sqName(s.from)}→${sqName(s.to)}${forced}, ${s.dome ? "dome" : "build"} ${sqName(s.build)}`;
    }
    case "move":
      return `${sqName(s.from)}→${sqName(s.to)}${s.win ? " wins" : ""}`;
    case "build":
      return `build ${sqName(s.build)}`;
    case "dome":
      return `dome ${sqName(s.build)}`;
    case "zeus":
      return `${sqName(s.from)}→${sqName(s.to)}, build under ${sqName(s.to)}`;
    case "force":
      return `force ${sqName(s.forced_from)}→${sqName(s.forced_to)}`;
    case "square":
      if (s.removed_block !== undefined) return `remove block ${sqName(s.removed_block)}`;
      if (s.forced_from !== undefined) {
        return s.forced_to !== undefined && s.forced_to !== null
          ? `drag ${sqName(s.forced_from)}→${sqName(s.forced_to)}`
          : `remove worker ${sqName(s.forced_from)}`;
      }
      return `dome ${sqName(s.square)}`;
    case "pass":
      return "end turn";
    default:
      return s.kind || "—";
  }
}

function logMove(msg) {
  const logEl = $("log");
  const li = document.createElement("li");
  li.textContent = msg;
  logEl.prepend(li);
}

function setEngineReadyChip() {
  const chip = $("engine-chip");
  if (!chip) return;
  chip.textContent = "Wasm ready";
  chip.className = "chip ready";
}

async function doAction(action) {
  const epoch = ++runEpoch;
  const hSide = activeHumanSide();
  const descBefore = view && view.legal_actions ? view.legal_actions.find((a) => a.action === action) : null;
  busy = true;
  resetSel();
  hintAction = null;
  activePvRank = null;
  $("hint-banner").classList.add("hidden");
  showHexStatus("");

  try {
    const state = await sendWorker("apply_action", { action });
    if (epoch !== runEpoch) return;
    if (descBefore) {
      logMove(`You (${seatColor(hSide)}): ${formatStep(descBefore)}`);
      lastSubTurnWorkerSq = descBefore.to !== undefined ? descBefore.to : (descBefore.from ?? null);
    } else {
      lastSubTurnWorkerSq = null;
    }
    view = state;
    if (!view.in_sub_turn) lastSubTurnWorkerSq = null;
    showError("");
    renderBoard();

    if (!view.game_over && isBotSeat(view.current_player)) {
      await runAiTurn(epoch);
    }
  } catch (err) {
    if (epoch === runEpoch) showError(err.message || String(err));
  } finally {
    if (epoch === runEpoch) {
      busy = false;
      setEngineReadyChip();
      renderBoard();
    }
  }
}

async function runAiTurn(epoch, { singleStep = false } = {}) {
  while (
    epoch === runEpoch &&
    view &&
    !view.game_over &&
    isBotSeat(view.current_player) &&
    (singleStep || !botsPaused || gameMode !== "bots")
  ) {
    const mover = view.current_player;
    const timeMs = seatBudgetMs(mover);
    const chip = $("engine-chip");
    chip.textContent = `Searching (${seatColor(mover)} · ${timeMs} ms)…`;
    chip.className = "chip thinking";
    renderBoard();

    const { search, state } = await sendWorker("ai_turn", { time_ms: timeMs });
    if (epoch !== runEpoch) return;

    view = state;
    lastAnalysis = search;
    if (search && search.steps && search.steps.length) {
      const prefix = gameMode === "bots"
        ? `${seatColor(mover)} (${timeMs} ms)`
        : `Bot (${seatColor(mover)} · ${timeMs} ms)`;
      logMove(`${prefix}: ${search.steps.map(formatStep).join(", ")}`);
    }

    setEngineReadyChip();
    renderBoard();

    if (singleStep) break;
    // In interactive browsers, yield 1 paint frame between Bot-vs-Bot turns so each move renders smoothly
    // and Pause clicks register cleanly without blocking; skipped in headless Node test VMs.
    if (gameMode === "bots" && !botsPaused && !view.game_over && typeof window.requestAnimationFrame === "function") {
      await new Promise((r) => window.requestAnimationFrame(() => setTimeout(r, 40)));
    }
  }
}

async function resumeOrStepBots({ singleStep = false } = {}) {
  if (busy || !view || view.game_over || !isBotSeat(view.current_player)) return;
  const epoch = ++runEpoch;
  busy = true;
  hintAction = null;
  activePvRank = null;
  try {
    await runAiTurn(epoch, { singleStep });
  } catch (err) {
    if (epoch === runEpoch) showError(err.message || String(err));
  } finally {
    if (epoch === runEpoch) {
      busy = false;
      setEngineReadyChip();
      renderBoard();
    }
  }
}

function syncControlsFromState() {
  if (!view) return;
  $("p0-god").value = String(view.gods[0]);
  $("p1-god").value = String(view.gods[1]);
}

async function startNewGame() {
  const epoch = ++runEpoch;
  busy = true;
  resetSel();
  hintAction = null;
  activePvRank = null;
  lastAnalysis = null;
  $("hint-banner").classList.add("hidden");
  $("log").replaceChildren();
  showHexStatus("");
  syncSeatLabels();

  const god0 = parseInt($("p0-god").value, 10) || 0;
  const god1 = parseInt($("p1-god").value, 10) || 0;

  try {
    const state = await sendWorker("new_game", { god0, god1 });
    if (epoch !== runEpoch) return;
    view = state;
    showError("");
    renderBoard();

    if (!view.game_over && isBotSeat(view.current_player) && !(gameMode === "bots" && botsPaused)) {
      await runAiTurn(epoch);
    }
  } catch (err) {
    if (epoch === runEpoch) showError(err.message || String(err));
  } finally {
    if (epoch === runEpoch) {
      busy = false;
      setEngineReadyChip();
      renderBoard();
    }
  }
}

async function undo() {
  if (busy || !view || !view.can_undo) return;
  const epoch = ++runEpoch;
  if (gameMode === "bots") botsPaused = true;
  busy = true;
  resetSel();
  hintAction = null;
  activePvRank = null;
  showHexStatus("");

  try {
    let state = await sendWorker("undo");
    if (gameMode !== "bots") {
      const hSide = activeHumanSide();
      while (state && state.can_undo && !state.game_over && state.current_player !== hSide) {
        state = await sendWorker("undo");
      }
    }
    if (epoch !== runEpoch) return;
    view = state;
    syncControlsFromState();
    showError("");
    renderBoard();
  } catch (err) {
    if (epoch === runEpoch) showError(err.message || String(err));
  } finally {
    if (epoch === runEpoch) {
      busy = false;
      renderBoard();
    }
  }
}

async function requestHint() {
  if (busy || !humanToMove()) return;
  const epoch = ++runEpoch;
  busy = true;
  const banner = $("hint-banner");
  banner.textContent = "Searching candidate moves…";
  banner.classList.remove("hidden");
  renderBoard();

  try {
    const timeMs = seatBudgetMs(view.current_player);
    const multiPv = parseInt($("multipv").value, 10) || 3;
    const res = await sendWorker("hint", { time_ms: timeMs, multi_pv: multiPv });
    if (epoch !== runEpoch) return;
    lastAnalysis = res;
    if (res && res.steps && res.steps.length) {
      const plan = res.steps.map(formatStep).join(", ");
      hintAction = res.steps[0];
      activePvRank = 1;
      banner.textContent = `Engine suggests ${plan} (${fmtScore(res.value)}, depth ${res.depth}).`;
    }
  } catch (err) {
    if (epoch === runEpoch) banner.textContent = `Hint error: ${err.message}`;
  } finally {
    if (epoch === runEpoch) {
      busy = false;
      renderBoard();
    }
  }
}

async function runAnalyze() {
  if (busy || !view || view.game_over) return;
  const epoch = ++runEpoch;
  busy = true;
  renderBoard();

  try {
    const timeMs = seatBudgetMs(view.current_player);
    const multiPv = parseInt($("multipv").value, 10) || 3;
    const res = await sendWorker("analyze", { time_ms: timeMs, multi_pv: multiPv });
    if (epoch !== runEpoch) return;
    lastAnalysis = res;
    if (res && res.lines && res.lines.length) {
      activePvRank = res.lines[0].rank;
      hintAction = res.lines[0].move_desc;
    }
    showError("");
  } catch (err) {
    if (epoch === runEpoch) showError(err.message || String(err));
  } finally {
    if (epoch === runEpoch) {
      busy = false;
      renderBoard();
    }
  }
}

async function copyHex() {
  const hex = ($("state-hex").value || "").trim();
  if (!hex) return;
  try {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(hex);
      showHexStatus("Copied");
    } else {
      showHexStatus("Select & copy");
    }
  } catch (_e) {
    showHexStatus("Copy blocked");
  }
}

async function loadHex() {
  const hex = ($("state-hex").value || "").trim();
  if (!hex) {
    showError("Paste a 102-character state hex string first.");
    return;
  }
  const epoch = ++runEpoch;
  if (gameMode === "bots") botsPaused = true;
  busy = true;
  resetSel();
  hintAction = null;
  activePvRank = null;
  lastAnalysis = null;
  try {
    const state = await sendWorker("set_hex", { hex });
    if (epoch !== runEpoch) return;
    view = state;
    syncControlsFromState();
    showError("");
    showHexStatus("Loaded");
    if (!view.game_over && isBotSeat(view.current_player) && !(gameMode === "bots" && botsPaused)) {
      await runAiTurn(epoch);
    }
  } catch (err) {
    if (epoch === runEpoch) showError(err.message || String(err));
  } finally {
    if (epoch === runEpoch) {
      busy = false;
      setEngineReadyChip();
      renderBoard();
    }
  }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

function populateGodSelects() {
  for (const id of ["p0-god", "p1-god"]) {
    const selEl = $(id);
    selEl.replaceChildren();
    for (const [label, isIntro] of [["Standard & intro (11)", true], ["Pantheon (33)", false]]) {
      const grp = document.createElement("optgroup");
      grp.label = label;
      for (const g of GOD_LIST.filter((x) => x.intro === isIntro)) {
        const opt = document.createElement("option");
        opt.value = String(g.id);
        opt.textContent = g.name;
        grp.appendChild(opt);
      }
      selEl.appendChild(grp);
    }
    selEl.value = "0";
  }
}

function initEventHandlers() {
  $("board").addEventListener("click", (e) => {
    const t = e.target.closest("[data-target-sq], [data-sq]");
    if (!t) return;
    const sqAttr = t.getAttribute("data-target-sq") ?? t.getAttribute("data-sq");
    if (sqAttr !== null) {
      onSquareClick(parseInt(sqAttr, 10));
    }
  });

  $("new-game").onclick = () => {
    botsPaused = false;
    startNewGame();
  };
  $("pause-bots").onclick = () => {
    if (gameMode !== "bots") return;
    botsPaused = !botsPaused;
    syncSeatLabels();
    if (!botsPaused && !busy && view && !view.game_over) {
      resumeOrStepBots({ singleStep: false });
    } else if (!busy) {
      renderBoard();
    }
  };
  $("step-bot").onclick = () => {
    if (gameMode !== "bots" || !botsPaused || busy || !view || view.game_over) return;
    resumeOrStepBots({ singleStep: true });
  };
  $("undo").onclick = undo;
  $("hint").onclick = requestHint;
  $("analyze").onclick = runAnalyze;
  $("copy-hex").onclick = copyHex;
  $("load-hex").onclick = loadHex;

  for (const id of ["p0-ms", "p1-ms"]) {
    const el = $(id);
    if (el) {
      el.onchange = () => {
        if (!busy && view) updateChrome();
      };
    }
  }

  const segBtns = $("mode-seg").querySelectorAll("button");
  segBtns.forEach((btn) => {
    btn.onclick = () => {
      segBtns.forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      gameMode = btn.getAttribute("data-mode") || "p0";
      botsPaused = false;
      syncSeatLabels();
      startNewGame();
    };
  });
  syncSeatLabels();
}

async function boot() {
  populateGodSelects();
  initEventHandlers();

  worker = new Worker("worker.js");
  worker.onmessage = (e) => {
    const { id, payload, error } = e.data;
    if (pending.has(id)) {
      const { resolve, reject } = pending.get(id);
      pending.delete(id);
      if (error) reject(new Error(error));
      else resolve(payload);
    }
  };

  const chip = $("engine-chip");
  try {
    await sendWorker("init");
    setEngineReadyChip();
    await startNewGame();
  } catch (err) {
    chip.textContent = "Load error";
    chip.className = "chip crimson";
    showError("Could not load Wasm engine: " + err.message);
  }
}

window.addEventListener("DOMContentLoaded", boot);
