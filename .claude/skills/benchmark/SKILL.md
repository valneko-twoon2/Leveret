---
name: benchmark
description: Measure performance before and after a change and report the numbers. Use for changes to git access, parsing, lists, diffs, startup or a new screen, and when asked whether something got slower.
---

# Benchmark

1. **New screen**: wrap its loading in `perf.view('<area>.<what>', ...)` and add a step to `frontend/src/lib/bench.ts`.
   Long backend work uses `perf::Span` / `perf::time`. Sample names never contain paths, arguments or tokens.
2. **Before the change** (on `main`, if there is no fresh baseline): `scripts\benchmark.bat -SaveBaseline`.
3. **After the change**: `scripts\benchmark.bat` (add `-Runs 10` when numbers are noisy). It compares with
   `artifacts\perf\baseline.json` and prints median / p95 per step.
4. **Budgets (p95)**: screen 100 ms, round trip 200 ms, git process 300 ms, GitHub 1500 ms, startup 1500 ms,
   main-thread task 50 ms. `-Strict` fails on over-budget or clearly slower steps.
5. **Report** the steps that changed (before -> after) in the summary. If the machine was busy and results are
   noise, say so instead of claiming an improvement.

While developing, `scripts\dev.bat` writes samples to `%LOCALAPPDATA%\com.leveret.desktop\logs\perf.jsonl`;
`Ctrl+Shift+D` opens the performance panel.

Build times are measured too: compare a rebuild with cached dependencies (touch one source file), not a cold build.
