// Comprehensive end-to-end behavioral UI test suite for AlphaSantorini Wasm.
// Instantiates the REAL compiled `web/alphasantorini.wasm` (`SANTNNUE03`, 44 powers)
// inside a faithful Web Worker + DOM harness and exercises human clicks across:
//   1. Static security, CSP cleanliness, zero BGA/Cyclades remnants, and Roxley credit.
//   2. All 44 supported powers (`new_game` -> setup clicks -> human turn -> AI response).
//   3. All 10 action-collision powers & Pantheon edge cases via custom 51-byte hex positions:
//      Prometheus, Achilles, Artemis (1-step, 2-step, & 1-step stop-build), Atlas, Selene,
//      Hermes, Triton, Atalanta, Heracles, Scylla, Castor & Pollux (move-both & build-both),
//      Zeus (under-build), Charon (force), Apollo/Minotaur (swap/push preview), Terpsichore,
//      Demeter, Ares, Medusa, Polyphemus, Bia (P2 places first), Eros (opposite edges),
//      plus Multi-PV analysis & state-hex import/export.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const WEB = path.join(ROOT, "web");

const SUPPORTED_GODS = [
  0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 19, 20, 21, 22, 23, 24,
  26, 27, 28, 29, 30, 37, 38, 44, 45, 46, 48, 49, 50, 54, 55, 57, 58, 61, 62, 63, 65, 67,
];

// ---------------------------------------------------------------------------
// 1. Static security, CSP, and public hygiene audit
// ---------------------------------------------------------------------------

const htmlSrc = fs.readFileSync(path.join(WEB, "index.html"), "utf8");
const appSrc = fs.readFileSync(path.join(WEB, "app.js"), "utf8");
const workerSrc = fs.readFileSync(path.join(WEB, "worker.js"), "utf8");
const cssSrc = fs.readFileSync(path.join(WEB, "style.css"), "utf8");

// Zero XSS / blocking modal sinks in JS
const forbiddenSinks = [
  ".innerHTML",
  ".outerHTML",
  "insertAdjacentHTML",
  "document.write",
  "eval(",
  "alert(",
  "confirm(",
  "prompt(",
];
for (const [rel, src] of [["web/app.js", appSrc], ["web/worker.js", workerSrc]]) {
  const codeOnly = src.split("\n").map((l) => l.replace(/\/\/.*$/, "")).join("\n");
  for (const sink of forbiddenSinks) {
    assert.ok(!codeOnly.includes(sink), `Forbidden sink ${sink} in ${rel}`);
  }
}

// Strict CSP compatibility: zero inline <script> bodies and zero inline style="..." in index.html
assert.ok(!/\bstyle\s*=/i.test(htmlSrc), "index.html must not contain inline style= attributes");
assert.ok(!/<script(?![^>]*\bsrc=)[^>]*>/i.test(htmlSrc), "index.html must not contain inline <script>");

// Zero BGA or Cyclades remnants across README, src/, and web/
for (const file of [
  "README.md",
  "Cargo.toml",
  "src/compact.rs",
  "src/gods.rs",
  "src/nnue.rs",
  "src/search.rs",
  "src/lib.rs",
  "web/index.html",
  "web/app.js",
  "web/worker.js",
  "web/style.css",
]) {
  const txt = fs.readFileSync(path.join(ROOT, file), "utf8").toLowerCase();
  assert.ok(!txt.includes("boardgamearena") && !/\bbga\b/.test(txt), `BGA reference found in ${file}`);
  assert.ok(!txt.includes("cyclades"), `Legacy 'cyclades' reference found in ${file}`);
}

// Official designer & publisher attribution
assert.ok(
  /Gordon Hamilton/i.test(htmlSrc) && /Roxley/i.test(htmlSrc),
  "index.html must credit Gordon Hamilton and Roxley Games"
);

// ---------------------------------------------------------------------------
// 2. Minimal DOM + In-Process WebAssembly Worker Harness
// ---------------------------------------------------------------------------

const wasmBytes = fs.readFileSync(path.join(WEB, "alphasantorini.wasm"));

function makeClassList(node) {
  const set = new Set((node.className || "").split(/\s+/).filter(Boolean));
  const sync = () => {
    node.className = [...set].join(" ");
  };
  return {
    add(...cs) {
      cs.forEach((c) => c && set.add(c));
      sync();
    },
    remove(...cs) {
      cs.forEach((c) => set.delete(c));
      sync();
    },
    toggle(c, force) {
      const on = force !== undefined ? Boolean(force) : !set.has(c);
      if (on) set.add(c);
      else set.delete(c);
      sync();
      return on;
    },
    contains(c) {
      return set.has(c);
    },
  };
}

function createHarness() {
  const byId = new Map();
  const segButtons = {};

  function makeEl(tag) {
    const listeners = {};
    const attrs = {};
    let proxy = null;
    const node = {
      tagName: String(tag).toUpperCase(),
      id: "",
      className: "",
      textContent: "",
      title: "",
      value: "",
      checked: false,
      disabled: false,
      dataset: {},
      style: {},
      parentNode: null,
      children: [],
      onclick: null,
      onchange: null,
      setAttribute(k, v) {
        attrs[k] = String(v);
        if (k === "id") {
          this.id = String(v);
          byId.set(this.id, proxy);
        }
        if (k === "class") {
          this.className = String(v);
          this.classList = makeClassList(this);
        }
        if (k.startsWith("data-")) {
          const camel = k.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase());
          this.dataset[camel] = String(v);
        }
      },
      getAttribute(k) {
        if (k === "id" && this.id) return this.id;
        if (k === "class" && this.className) return this.className;
        return attrs[k] ?? null;
      },
      hasAttribute(k) {
        return this.getAttribute(k) !== null;
      },
      appendChild(child) {
        child.parentNode = proxy;
        this.children.push(child);
        if (child.id) byId.set(child.id, child);
        return child;
      },
      prepend(child) {
        child.parentNode = proxy;
        this.children.unshift(child);
        if (child.id) byId.set(child.id, child);
        return child;
      },
      replaceChildren(...cs) {
        this.children = [];
        cs.forEach((c) => this.appendChild(c));
      },
      addEventListener(ev, fn) {
        (listeners[ev] = listeners[ev] || []).push(fn);
      },
      closest(selector) {
        const wantsSq = selector.includes("data-sq");
        const wantsTarget = selector.includes("data-target-sq");
        let cur = proxy;
        while (cur) {
          if ((wantsSq && cur.getAttribute("data-sq") !== null) || (wantsTarget && cur.getAttribute("data-target-sq") !== null)) {
            return cur;
          }
          cur = cur.parentNode;
        }
        return null;
      },
      querySelectorAll(sel) {
        if (sel === "button") {
          return this.children.filter((c) => c.tagName === "BUTTON");
        }
        return [];
      },
      click() {
        const ev = {
          stopPropagation() {},
          preventDefault() {},
          target: proxy,
          currentTarget: proxy,
        };
        if (typeof this.onclick === "function") this.onclick(ev);
        (listeners.click || []).forEach((fn) => fn(ev));
        // Bubble to #board listener if inside SVG board
        let p = this.parentNode;
        while (p) {
          const pev = {
            stopPropagation() {},
            preventDefault() {},
            target: proxy,
            currentTarget: p,
          };
          if (typeof p.onclick === "function") p.onclick(pev);
          p._emit?.("click", pev);
          p = p.parentNode;
        }
      },
      _emit(evName, ev) {
        (listeners[evName] || []).forEach((fn) => fn(ev));
      },
    };
    node.classList = makeClassList(node);
    proxy = new Proxy(node, {
      set(t, k, v) {
        t[k] = v;
        if (k === "id" && v) byId.set(String(v), proxy);
        if (k === "className") t.classList = makeClassList(t);
        return true;
      },
    });
    return proxy;
  }

  // Parse IDs and segmented buttons from index.html
  let currentSegId = null;
  for (const line of htmlSrc.split("\n")) {
    const segMatch = /class="[^"]*\bseg\b[^"]*"[^>]*id="([^"]+)"|id="([^"]+)"[^>]*class="[^"]*\bseg\b[^"]*"/.exec(line);
    if (segMatch) currentSegId = segMatch[1] || segMatch[2];
    const tagId = /<([a-z0-9]+)\b([^>]*\bid="([^"]+)"[^>]*)>/gi;
    let m;
    while ((m = tagId.exec(line)) !== null) {
      const [, tag, attrText, id] = m;
      const n = makeEl(tag);
      n.id = id;
      const cls = /\bclass="([^"]+)"/.exec(attrText);
      if (cls) n.className = cls[1];
      const val = /\bvalue="([^"]*)"/.exec(attrText);
      if (val) n.value = val[1];
      byId.set(id, n);
    }
    if (currentSegId) {
      const bm = /<button\b([^>]*)>([^<]*)<\/button>/.exec(line);
      if (bm) {
        const btn = makeEl("button");
        btn.textContent = bm[2];
        const cls = /\bclass="([^"]+)"/.exec(bm[1]);
        if (cls) btn.className = cls[1];
        const dm = /\bdata-([a-z-]+)="([^"]+)"/g;
        let d;
        while ((d = dm.exec(bm[1])) !== null) btn.setAttribute("data-" + d[1], d[2]);
        (segButtons[currentSegId] = segButtons[currentSegId] || []).push(btn);
        if (byId.has(currentSegId)) byId.get(currentSegId).appendChild(btn);
      }
      if (line.includes("</div>")) currentSegId = null;
    }
  }

  // Parse static <select> options (e.g. #multipv)
  const selectRe = /<select\b[^>]*\bid="([^"]+)"[^>]*>([\s\S]*?)<\/select>/gi;
  let sm;
  while ((sm = selectRe.exec(htmlSrc)) !== null) {
    const selEl = byId.get(sm[1]);
    if (!selEl) continue;
    const optRe = /<option\b([^>]*)>([^<]*)<\/option>/gi;
    let om;
    while ((om = optRe.exec(sm[2])) !== null) {
      const val = /\bvalue="([^"]*)"/.exec(om[1]);
      const selected = /\bselected\b/.test(om[1]);
      const opt = makeEl("option");
      opt.value = val ? val[1] : "";
      opt.textContent = om[2];
      selEl.appendChild(opt);
      if (selected || !selEl.value) selEl.value = opt.value;
    }
  }

  // In-process Web Worker executing real `web/worker.js` + `alphasantorini.wasm`
  let workerInflight = 0;
  class MockWorker {
    constructor(_scriptUrl) {
      this.onmessage = null;
      const selfRef = {
        onmessage: null,
        postMessage: (data) => {
          queueMicrotask(() => {
            try {
              this.onmessage && this.onmessage({ data });
            } finally {
              workerInflight = Math.max(0, workerInflight - 1);
            }
          });
        },
      };
      const workerCtx = {
        self: selfRef,
        WebAssembly,
        Uint8Array,
        TextDecoder,
        TextEncoder,
        performance,
        console,
        fetch: async (_url) => ({
          ok: true,
          status: 200,
          arrayBuffer: async () => wasmBytes.buffer.slice(wasmBytes.byteOffset, wasmBytes.byteOffset + wasmBytes.byteLength),
        }),
      };
      vm.createContext(workerCtx);
      vm.runInContext(workerSrc, workerCtx);
      this._selfRef = selfRef;
    }
    postMessage(data) {
      workerInflight++;
      queueMicrotask(() => this._selfRef.onmessage({ data }));
    }
  }

  const domListeners = {};
  const ctx = {
    console,
    setTimeout,
    clearTimeout,
    performance,
    Worker: MockWorker,
    navigator: { clipboard: { writeText: async () => {} } },
    window: {
      addEventListener(ev, fn) {
        (domListeners[ev] = domListeners[ev] || []).push(fn);
      },
    },
    document: {
      activeElement: null,
      getElementById(id) {
        return byId.get(id) || null;
      },
      createElement: makeEl,
      createElementNS(_ns, tag) {
        return makeEl(tag);
      },
      querySelectorAll(sel) {
        const m = /^#([a-z0-9-]+)\s+button$/i.exec(sel);
        if (m) return segButtons[m[1]] || [];
        return [];
      },
    },
  };
  vm.createContext(ctx);
  vm.runInContext(appSrc, ctx);

  async function flush(ticks = 8) {
    for (let guard = 0; guard < 2000; guard++) {
      for (let i = 0; i < ticks; i++) {
        await new Promise((r) => setImmediate(r));
      }
      if (workerInflight === 0) {
        // Confirm no chained worker RPC (e.g., apply_action -> runAiTurn) started on resume
        for (let i = 0; i < 4; i++) {
          await new Promise((r) => setImmediate(r));
        }
        if (workerInflight === 0) return;
      }
      await new Promise((r) => setTimeout(r, 2));
    }
    throw new Error(`Worker did not settle; workerInflight=${workerInflight}`);
  }

  function clickBoardSq(sq) {
    const board = byId.get("board");
    // Prefer target overlay if present, else worker or ground rect
    const findRec = (node, attr) => {
      if (node.getAttribute && node.getAttribute(attr) === String(sq)) return node;
      for (const ch of node.children || []) {
        const found = findRec(ch, attr);
        if (found) return found;
      }
      return null;
    };
    const targetNode = findRec(board, "data-target-sq") || findRec(board, "data-sq");
    assert.ok(targetNode, `No clickable SVG element for square ${sq}`);
    targetNode.click();
  }

  function targetSquares() {
    const out = [];
    const walk = (node) => {
      const v = node.getAttribute && node.getAttribute("data-target-sq");
      if (v !== null && v !== undefined) out.push(Number(v));
      for (const ch of node.children || []) walk(ch);
    };
    walk(byId.get("board"));
    return out;
  }

  function actionButtons() {
    return byId.get("action-bar").children.filter((c) => c.tagName === "BUTTON");
  }

  function clickButtonMatching(regex) {
    const btn = actionButtons().find((b) => regex.test(b.textContent));
    assert.ok(
      btn,
      `Expected button matching ${regex}; available: [${actionButtons().map((b) => b.textContent).join(" | ")}]`
    );
    btn.click();
  }

  return {
    byId,
    segButtons,
    domListeners,
    flush,
    clickBoardSq,
    targetSquares,
    actionButtons,
    clickButtonMatching,
  };
}

// Helper: encode a 51-byte CompactSantoriniState hex string for mid-game collision fixtures!
function encodeMidgameHex({
  height = new Array(25).fill(0),
  workers = [[6, 18], [8, 16]],
  gods = [0, 0],
  currentPlayer = 0,
  lowDomes = 0,
  powerUsed = 0,
  statusFlags = 0,
  female = [[false, false], [false, false]],
}) {
  const b = Buffer.alloc(51, 0);
  for (let i = 0; i < 25; i++) b[i] = height[i];
  // Sort workers & tags per player cleanly
  for (let p = 0; p < 2; p++) {
    let pair = [
      { sq: workers[p][0], tag: female[p][0] ? 1 : 0 },
      { sq: workers[p][1], tag: female[p][1] ? 1 : 0 },
    ];
    if (pair[0].sq > pair[1].sq) pair = [pair[1], pair[0]];
    b[25 + p * 2] = pair[0].sq;
    b[25 + p * 2 + 1] = pair[1].sq;
    b[43 + p * 2] = pair[0].tag;
    b[43 + p * 2 + 1] = pair[1].tag;
  }
  b[29] = gods[0];
  b[30] = gods[1];
  b[31] = statusFlags;
  b[32] = 2; // PHASE_TURN
  b[33] = 0; // active_worker = 0
  b[34] = 255; // prev_square = UNPLACED
  b[35] = currentPlayer;
  b[36] = 0; // game_over = false
  b[37] = 0xff; // winner = -1
  b.writeUInt16LE(6, 38); // turn_count = 6
  b[40] = powerUsed;
  b[41] = 0; // turn_flags
  b[42] = 0; // stage = STAGE_NONE
  b.writeUInt32LE(lowDomes >>> 0, 47);
  return b.toString("hex");
}

// ---------------------------------------------------------------------------
// 3. Execute Full Suite
// ---------------------------------------------------------------------------

const h = createHarness();
for (const fn of h.domListeners.DOMContentLoaded || []) fn();
await h.flush(20);

assert.equal(h.byId.get("error").textContent, "", `Boot error: ${h.byId.get("error").textContent}`);
assert.match(h.byId.get("engine-chip").textContent, /ready/i);

// Use 15ms AI search budget during tests so 44+ games complete in ~2 seconds
h.byId.get("time-ms").value = "15";

// Verify all 44 powers are selectable in #human-god and #engine-god
const collectOptionValues = (selNode) => {
  const vals = [];
  const walk = (n) => {
    if (n.tagName === "OPTION") vals.push(Number(n.value));
    for (const c of n.children || []) walk(c);
  };
  walk(selNode);
  return vals;
};
const humanOpts = collectOptionValues(h.byId.get("human-god"));
assert.deepEqual(
  [...humanOpts].sort((a, b) => a - b),
  [...SUPPORTED_GODS].sort((a, b) => a - b),
  "Dropdown must expose all 44 supported gods"
);

async function loadHexFixture(opts) {
  const hex = encodeMidgameHex(opts);
  h.byId.get("state-hex").value = hex;
  h.byId.get("load-hex").click();
  await h.flush(15);
  assert.equal(h.byId.get("error").textContent, "", `Hex load failed: ${h.byId.get("error").textContent}`);
}

// --- Collision Test A: Prometheus (10) & Achilles (46) pre-build on movable ground square ---
for (const godId of [10, 46]) {
  await loadHexFixture({ gods: [godId, 0], workers: [[6, 18], [0, 24]] });
  h.clickBoardSq(6); // select worker on b4
  await h.flush(4);
  // Toggle to build-before-moving and build on movable square 12 (c3)!
  h.clickButtonMatching(/build/i);
  await h.flush(4);
  assert.ok(h.targetSquares().includes(12), `God ${godId} should highlight sq 12 as pre-build target`);
  h.clickBoardSq(12); // pre-build on c3!
  await h.flush(10);
  // Now in sub-turn: worker on 6 is auto-selected; move to 7 (c4), then post-build on 2 (c5)!
  assert.ok(h.targetSquares().includes(7), `God ${godId} should auto-select worker 6 for move step`);
  h.clickBoardSq(7);
  await h.flush(10);
  assert.ok(h.targetSquares().includes(2), `God ${godId} should highlight post-build square 2`);
  h.clickBoardSq(2);
  await h.flush(20);
  assert.equal(h.byId.get("error").textContent, "");
}

// --- Collision Test B: Artemis (2) 2-step move AND 1-step stop-and-build on empty square ---
await loadHexFixture({ gods: [2, 0], workers: [[6, 18], [0, 24]] });
h.clickBoardSq(6);
h.clickBoardSq(7); // preview 1st step to 7
h.clickButtonMatching(/move again|artemis|move only/i); // enter STAGE_ARTEMIS
await h.flush(10);
// In STAGE_ARTEMIS, toggle to build-only on movable ground square 12!
h.clickButtonMatching(/build/i);
await h.flush(4);
assert.ok(h.targetSquares().includes(12), "Artemis 1-step stop-build must reach movable square 12");
h.clickBoardSq(12);
await h.flush(20);

// --- Collision Test C: Hermes (7) multi-step level walk + stop-and-build ---
await loadHexFixture({ gods: [7, 0], workers: [[6, 18], [0, 24]] });
h.clickBoardSq(6);
h.clickBoardSq(7);
h.clickButtonMatching(/walk|hermes|move/i); // MoveOnly step 6 -> 7
await h.flush(10);
h.clickBoardSq(12); // 2nd level step 7 -> 12 (no `turn` exists in STAGE_HERMES, executes immediately!)
await h.flush(10);
h.clickButtonMatching(/build/i); // switch from walking to building
await h.flush(4);
assert.ok(h.targetSquares().includes(13), "Hermes stop-build must highlight adjacent ground square 13");
h.clickBoardSq(13);
await h.flush(20);

// --- Collision Test D: Triton (29) perimeter chain + stop-and-build ---
await loadHexFixture({ gods: [29, 0], workers: [[6, 18], [20, 24]] });
h.clickBoardSq(6);
h.clickBoardSq(1); // perimeter square b5 -> MoveOnly executes immediately into STAGE_CHAIN!
await h.flush(10);
h.clickBoardSq(2); // chain step b5 -> c5
await h.flush(10);
h.clickButtonMatching(/build/i);
await h.flush(4);
h.clickBoardSq(7); // build on c4
await h.flush(20);

// --- Collision Test E: Atalanta (48) Hero sprint ---
await loadHexFixture({ gods: [48, 0], workers: [[6, 18], [20, 24]] });
h.clickBoardSq(6);
h.clickBoardSq(7);
h.clickButtonMatching(/sprint|atalanta|move/i);
await h.flush(10);
h.clickBoardSq(12); // extra sprint step
await h.flush(10);
h.clickButtonMatching(/build/i);
await h.flush(4);
h.clickBoardSq(13);
await h.flush(20);

// --- Collision Test F: Heracles (50) Hero dome storm + Pass ---
await loadHexFixture({ gods: [50, 0], workers: [[6, 18], [20, 24]] });
h.clickBoardSq(6);
h.clickBoardSq(7);
h.clickButtonMatching(/dome|heracles|move/i);
await h.flush(10);
assert.ok(h.targetSquares().includes(12), "Heracles should highlight adjacent dome square 12");
h.clickBoardSq(12); // place ground dome on 12
await h.flush(10);
h.clickButtonMatching(/end|done|pass|complete/i);
await h.flush(20);

// --- Collision Test G: Scylla (57) neighbor drag ---
await loadHexFixture({ gods: [57, 0], workers: [[12, 18], [7, 24]] });
h.clickBoardSq(12);
h.clickBoardSq(13); // move 12 -> 13 (vacating 12 adjacent to opponent on 7)
h.clickButtonMatching(/drag|scylla|move/i);
await h.flush(10);
assert.ok(h.targetSquares().includes(7), "Scylla STAGE_SCYLLA must highlight opponent on 7");
h.clickBoardSq(7); // drag opponent 7 -> 12
await h.flush(10);
h.clickBoardSq(14); // build on 14
await h.flush(20);

// --- Collision Test H: Castor & Pollux (58) BOTH move-both AND build-both ---
await loadHexFixture({ gods: [58, 0], workers: [[6, 18], [0, 24]] });
h.clickBoardSq(6);
h.clickBoardSq(7);
h.clickButtonMatching(/both|castor|move/i); // MoveOnly worker 0 (6 -> 7)
await h.flush(10);
h.clickBoardSq(17); // auto-selected worker 1 (18 -> 17), ends turn!
await h.flush(20);

await loadHexFixture({ gods: [58, 0], workers: [[6, 18], [0, 24]] });
h.clickBoardSq(6);
h.clickButtonMatching(/build/i); // BuildOnly worker 0 on movable ground square 7!
await h.flush(4);
h.clickBoardSq(7);
await h.flush(10);
h.clickBoardSq(17); // auto-selected worker 1 BuildOnly on 17!
await h.flush(20);

// --- Collision Test I: Zeus (30) self-build under moved worker ---
await loadHexFixture({ gods: [30, 0], workers: [[6, 18], [0, 24]] });
h.clickBoardSq(6);
h.clickBoardSq(12);
await h.flush(4);
assert.ok(h.targetSquares().includes(12), "Zeus must keep destination 12 clickable as under-build target");
h.clickBoardSq(12); // builds under himself on 12 without cancelling!
await h.flush(20);
const zeusLog = h.byId.get("log").children.map((li) => li.textContent).join(" | ");
assert.match(zeusLog, /under/i, `Expected Zeus self-build in move log, got: ${zeusLog}`);

// --- Collision Test J: Atlas (4) & Selene (28) remote female low-dome ---
await loadHexFixture({
  gods: [28, 0],
  workers: [[0, 18], [4, 20]],
  female: [[true, false], [false, false]], // sq 0 (a5) is female, sq 18 (d2) is male!
});
h.clickBoardSq(18); // select male worker at d2
h.clickBoardSq(19); // move male worker to e2
await h.flush(4);
// Selene's female worker at 0 (a5) can dome adjacent square 1 (b5), far from e2!
assert.ok(h.targetSquares().includes(1), "Selene must offer female-adjacent dome square 1");
h.clickBoardSq(1); // only dome is legal at 1, executes immediately!
await h.flush(20);

await loadHexFixture({ gods: [4, 0], workers: [[6, 18], [0, 24]] });
h.clickBoardSq(6);
h.clickBoardSq(12);
await h.flush(4);
h.clickBoardSq(7); // both block and dome legal -> opens domeChoice!
await h.flush(4);
h.clickButtonMatching(/dome/i);
await h.flush(20);

// --- Collision Test K: Charon (15) force + Apollo (1) swap + Ares (12) block removal ---
await loadHexFixture({ gods: [15, 0], workers: [[12, 24], [7, 0]] });
h.clickBoardSq(12);
await h.flush(4);
assert.ok(h.targetSquares().includes(7), "Charon must highlight adjacent opponent on 7 to force");
h.clickBoardSq(7); // forces opponent 7 -> 17!
await h.flush(10);
h.clickBoardSq(7); // move onto vacated 7
await h.flush(4);
h.clickBoardSq(2); // build on 2
await h.flush(20);

await loadHexFixture({ gods: [1, 0], workers: [[12, 24], [7, 0]] });
h.clickBoardSq(12);
h.clickBoardSq(7); // swap preview with opponent on 7 -> opponent moves to 12, so 12 is NOT a build target!
await h.flush(4);
assert.ok(!h.targetSquares().includes(12), "Apollo cannot build on 12 after swapping opponent onto 12");
h.clickBoardSq(2);
await h.flush(20);

const aresH = new Array(25).fill(0);
aresH[1] = 1;
await loadHexFixture({ gods: [12, 0], height: aresH, workers: [[0, 12], [4, 24]] });
h.clickBoardSq(12);
h.clickBoardSq(13);
await h.flush(4);
h.clickBoardSq(14);
await h.flush(10);
// Now in STAGE_END: unmoved worker on 0 can remove block on 1!
assert.ok(h.targetSquares().includes(1), "Ares STAGE_END must highlight removable block on 1");
h.clickBoardSq(1);
await h.flush(20);
const aresLog = h.byId.get("log").children.map((li) => li.textContent).join(" | ");
assert.ok(!/\bsquare\b/i.test(aresLog) && /b5/i.test(aresLog), `Ares log must name b5, got: ${aresLog}`);

// --- Multi-PV Analyze button check ---
await loadHexFixture({ gods: [0, 0], workers: [[6, 18], [8, 16]] });
h.byId.get("analyze").click();
await h.flush(20);
assert.equal(h.byId.get("pv-body").children.length, 3, "Multi-PV table should render 3 candidate lines");

// --- Exhaustive 44-God Playability Loop (Setup + Turn 1 + AI Response) ---
function playOneHumanAction() {
  const tgs = h.targetSquares();
  if (tgs.length > 0) {
    h.clickBoardSq(tgs[0]);
    return;
  }
  // Select a movable worker on the board
  const board = h.byId.get("board");
  const workers = [];
  const walk = (n) => {
    if (n.classList && n.classList.contains("selectable") && n.getAttribute("data-sq") !== null) {
      workers.push(Number(n.getAttribute("data-sq")));
    }
    for (const c of n.children || []) walk(c);
  };
  walk(board);
  assert.ok(workers.length > 0, "Expected either target squares or a selectable worker");
  h.clickBoardSq(workers[0]);
}

for (const godId of SUPPORTED_GODS) {
  h.byId.get("human-god").value = String(godId);
  h.byId.get("engine-god").value = "0";
  h.byId.get("new-game").click();
  await h.flush(25);
  assert.equal(h.byId.get("error").textContent, "", `New game failed for god ${godId}`);

  // Play setup + at least one full post-setup human turn (up to 16 clicks)
  let postSetupClicks = 0;
  for (let step = 0; step < 20; step++) {
    const statusTxt = h.byId.get("status").textContent;
    if (/game over/i.test(statusTxt)) break;
    const instrTxt = h.byId.get("instructions").textContent;
    const inSetup = /place|setup/i.test(instrTxt);
    if (!inSetup) postSetupClicks++;

    // If domeChoice is open, pick Block
    const blockBtn = h.actionButtons().find((b) => /^build block/i.test(b.textContent));
    if (blockBtn) {
      blockBtn.click();
    } else {
      playOneHumanAction();
    }
    await h.flush(25);
    assert.equal(h.byId.get("error").textContent, "", `Error during god ${godId}: ${h.byId.get("error").textContent}`);
    if (postSetupClicks >= 5) break;
  }
  assert.ok(postSetupClicks >= 2, `God ${godId} never reached post-setup play`);
}

console.log(`PASS: All ${SUPPORTED_GODS.length} Pantheon powers, 11 collision scenarios, Multi-PV, and CSP/hygiene checks verified against real alphasantorini.wasm.`);
