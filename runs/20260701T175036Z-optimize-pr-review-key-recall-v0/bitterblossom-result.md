I have now thoroughly reviewed the entire change. Here's my systematic assessment:

**Files changed:**
- `crates/threshold-core/src/optimization_target.rs` (new, 1459 lines) — core logic
- `crates/threshold-cli/src/main.rs` (+152 lines) — CLI entry point
- `crates/threshold-core/src/lib.rs` (+1 line) — module registration
- `docs/crucible-eval-optimization-contract.md` (new, 193 lines) — documentation
- Several design doc updates

**Specific items examined:**

1. **`summarize_candidates`**: `matched` is capped at `expected` via `.min(expected)`, preventing overcounting. Division-by-zero guarded for `point`, `task_recall`, `reward_mean`, and `wilson_interval`. Input validation checks for missing task/candidate combos and returns `Err`. ✅

2. **`wilson_interval`**: Standard Wilson score interval formula is correctly implemented. `n == 0` early-return prevents division by zero. ✅

3. **`build_headroom_probe`**: Verdict logic correctly handles all cases — `pass` (oracle reaches ceiling, null is floor, ranks exist), `saturated` (probe within 0.1 of oracle), `needs-review` (fallback). The `saturated` check correctly requires `oracle_pass` as a precondition. ✅

4. **`dispatch_bitterblossom`**: Both `Ok` and `Err` paths produce valid receipt JSON. The `canonicalize` call on `request_path` (a file that was just written) should always succeed, making the fallback a very low-probability safety net. ✅

5. **`cost_usd` handling**: Non-costless candidates with null/missing costs correctly set `known_cost = false`. NaN propagation in costs/rewards is possible but only affects report display, not correctness. ✅

6. **No concurrency**: Entirely single-threaded synchronous code — no data races possible. ✅

7. **No secret leakage**: The code declares `OPENROUTER_API_KEY` and `GH_TOKEN` as environment allowlist entries but never reads or writes their values. ✅

8. **Tests**: Three test cases cover normal pass, saturated, and broken-oracle scenarios with correct assertions. ✅

No logic errors, broken invariants, unhandled failure paths, concurrency bugs, data races, or off-by-ones found.

```json
{"verdict": "pass",
 "findings": []}
```
