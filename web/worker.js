// Web Worker hosting the AlphaSantorini WebAssembly engine (`SANTNNUE03`, 44 powers).
"use strict";

let wasm = null;

function readString(ptr) {
  const mem = new Uint8Array(wasm.memory.buffer);
  let end = ptr;
  while (mem[end] !== 0) end++;
  return new TextDecoder().decode(mem.subarray(ptr, end));
}

async function loadWasm() {
  const imports = {
    env: {
      js_now: () => performance.now(),
    },
  };
  const response = await fetch("alphasantorini.wasm?v=5");
  const buffer = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(buffer, imports);
  wasm = instance.exports;
  const ok = wasm.santorini_init();
  if (!ok) throw new Error("Failed to initialize AlphaSantorini SANTNNUE03 engine.");
}

self.onmessage = async (e) => {
  const { id, type, payload } = e.data;
  try {
    if (!wasm && type !== "init") {
      await loadWasm();
    }

    switch (type) {
      case "init": {
        await loadWasm();
        self.postMessage({ id, type: "ready" });
        break;
      }
      case "new_game": {
        const { god0, god1 } = payload;
        const ok = wasm.santorini_new_game(god0, god1);
        if (!ok) throw new Error(`Unsupported god pair: (${god0}, ${god1})`);
        const stateStr = readString(wasm.santorini_get_state());
        self.postMessage({ id, type: "state", payload: JSON.parse(stateStr) });
        break;
      }
      case "apply_action": {
        const { action } = payload;
        const ok = wasm.santorini_apply_action(action);
        if (!ok) throw new Error(`Illegal action: ${action}`);
        const stateStr = readString(wasm.santorini_get_state());
        self.postMessage({ id, type: "state", payload: JSON.parse(stateStr) });
        break;
      }
      case "ai_turn": {
        const { time_ms } = payload;
        const searchStr = readString(wasm.santorini_ai_full_turn(time_ms || 150));
        const stateStr = readString(wasm.santorini_get_state());
        self.postMessage({
          id,
          type: "ai_turn_result",
          payload: {
            search: JSON.parse(searchStr),
            state: JSON.parse(stateStr),
          },
        });
        break;
      }
      case "hint": {
        const { time_ms, multi_pv } = payload;
        const searchStr = readString(wasm.santorini_search(time_ms || 150, 0, multi_pv || 1));
        self.postMessage({
          id,
          type: "hint_result",
          payload: JSON.parse(searchStr),
        });
        break;
      }
      case "analyze": {
        const { time_ms, multi_pv } = payload;
        const searchStr = readString(wasm.santorini_search(time_ms || 150, 0, multi_pv || 3));
        self.postMessage({
          id,
          type: "analyze_result",
          payload: JSON.parse(searchStr),
        });
        break;
      }
      case "undo": {
        wasm.santorini_undo();
        const stateStr = readString(wasm.santorini_get_state());
        self.postMessage({ id, type: "state", payload: JSON.parse(stateStr) });
        break;
      }
      case "set_hex": {
        const bytes = new TextEncoder().encode(String(payload.hex || "").trim());
        const ptr = wasm.santorini_alloc(bytes.length);
        let ok = 0;
        try {
          new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
          ok = wasm.santorini_set_hex(ptr, bytes.length);
        } finally {
          wasm.santorini_dealloc(ptr, bytes.length);
        }
        if (!ok) throw new Error("Invalid 102-character state hex.");
        const stateStr = readString(wasm.santorini_get_state());
        self.postMessage({ id, type: "state", payload: JSON.parse(stateStr) });
        break;
      }
      default:
        throw new Error(`Unknown worker message type: ${type}`);
    }
  } catch (err) {
    self.postMessage({ id, type: "error", error: err.message || String(err) });
  }
};
