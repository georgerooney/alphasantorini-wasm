# AlphaSantorini (WebAssembly)

Standalone in-browser engine and arena for *Santorini* across Mortal rules, 10 introductory powers, and 33 Pantheon (2nd Edition) powers (44 total). Supports playing First (Navy), playing Second (Ochre), or watching **Bot vs. Bot** with independent per-seat powers, per-seat search budgets (`15–5,000 ms`), Pause/Resume, and single-turn Step.

Live demo: [lab.georgerooney.com/santorini](https://lab.georgerooney.com/santorini)

## Strength & Benchmarks

Running a 6-block AlphaZero ResNet over 200 MCTS simulations on client CPUs takes hundreds of milliseconds per move. To run inside a browser tab with no backend server, I distilled the trained ResNet + search teacher (`az-gods-v4`) into a quantized integer NNUE (`SANTNNUE03`, `nnue-gods-v4`) and paired it with a principal-variation alpha-beta search inside a Web Worker.

- **Standard (Mortal) rules**: 96.0% win rate against the `az-01` MCTS-200 baseline (95% Wilson CI [92.3%, 98.0%], `n = 200` paired games, 50 ms/move).
- **44-power Pantheon suite (`nnue-gods-v2`)**: 80.0% win rate on Pantheon-33 against `az-gods-v3` at 200 MCTS simulations (95% Wilson CI [74.0%, 84.9%], `n = 200` paired games, 50 ms/move) and 71.5% on the 10 introductory powers (95% Wilson CI [66.9%, 75.7%], `n = 400` paired games).
- **Reverse-distilled `SANTNNUE03` (`nnue-gods-v4`, shipped here)**: 57.3% win rate against `nnue-gods-v2` across `n = 2,800` paired games at equal 8,000-node budgets (95% Wilson CI [55.5%, 59.1%], +51.2 Elo, two-sided exact binomial `p < 1e-12`), and 54.3% against `nnue-gods-v3` (95% Wilson CI [52.4%, 56.1%], +29.9 Elo, `p = 6.0e-6`, `n = 2,800`).

## Architecture

| Component | Specification | Notes |
| :--- | :--- | :--- |
| State representation | 52-byte `CompactSantoriniState` (51-byte wire hex) | 17-stage explicit sub-turn state machine (`STAGE_*`), SplitMix64 Zobrist hashing |
| Evaluator | `SANTNNUE03` (`2,561 -> 2x256 -> 1`, 4 height buckets) | `int16` clipped-ReLU (`QA = 255, QB = 512`), `44 x 44 = 1,936` ordered god-pair interaction features, 1.25 MiB (`1,315,890` bytes) embedded via `include_bytes!` |
| Search | Single-threaded PVS alpha-beta + Multi-PV | Per-seat isolated transposition tables (`2 x 131,072` entries so asymmetric Bot-vs-Bot budgets never leak depths across players), killer/history heuristics, LMR, RFP, NMP, singular extensions, threat quiescence, and lossless `Hermes`/`Triton`/`Atalanta`/`Heracles` walk-cycle pruning |
| Browser runtime | `wasm32-unknown-unknown` + Web Worker | 1.48 MB raw (`907 KB` gzipped), no `SharedArrayBuffer` or `COOP`/`COEP` headers required, strict CSP (`script-src 'self'; style-src 'self'`) |

## Supported Powers (44)

- **Standard & introductory (11)**: Mortal, Apollo, Artemis, Athena, Atlas, Demeter, Hephaestus, Hermes, Minotaur, Pan, Prometheus.
- **Pantheon 2nd Edition (33)**: Achilles, Aphrodite, Ares, Asteria, Atalanta, Bellerophon, Bia, Castor & Pollux, Charon, Chronus, Eros, Hades, Harpies, Heracles, Hera, Hestia, Hippolyta, Hypnus, Iris, Limus, Maenads, Medusa, Pegasus, Persephone, Polyphemus, Poseidon, Scylla, Selene, Terpsichore, Theseus, Triton, Urania, Zeus.

## Building & Testing Locally

Prerequisites: Rust stable toolchain with the `wasm32-unknown-unknown` target, and Node.js 20+ for the end-to-end UI test suite.

```bash
rustup target add wasm32-unknown-unknown

# Run Rust unit tests & linter
cargo test
cargo clippy --all-targets -- -D warnings

# Compile WebAssembly engine and copy into web/
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/alphasantorini.wasm web/alphasantorini.wasm

# Run end-to-end 44-power + collision + CSP test suite against the compiled Wasm binary
node tests/wasm_all_44_gods_ui.mjs

# Serve locally
python3 -m http.server 8080 -d web
```

## Attribution

*Santorini* was designed by Gordon Hamilton and is published by [Roxley Games](https://roxley.com/products/santorini). This repository is an independent, non-commercial single-player AI research project with clean-room Rust/SVG code and no official affiliation with Roxley Games.
