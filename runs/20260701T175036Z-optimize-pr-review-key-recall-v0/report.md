# Optimizer Headroom Probe: pr-review-key-recall-v0

- Verdict: `pass`
- Budget cap: `$5.00`
- Crucible eval digest: `sha256:5cd166a6322dd6be5a06385fffb71ee2958256d9d1c17e5f9c7845c96c7e0cca`
- Source trials digest: `sha256:2c9c40bac177b178e40b1b900a8634aab2db649136f3a8bb65272b8bf01166fc`

| candidate | key recall | Wilson 95% CI | defects | reward mean | known cost | wall |
|---|---:|---:|---:|---:|---:|---:|
| null | 0.0000 | [0.0000, 0.2991] | 0/9 | 0.1667 | $0.0000 | 0 ms |
| oracle | 1.0000 | [0.7009, 1.0000] | 9/9 | 1.0000 | $0.0000 | 0 ms |
| probe-oneshot | 0.5556 | [0.2667, 0.8112] | 5/9 | 0.6000 | $0.0207 | 141543 ms |

## Sprites Dispatch

- Requested: `true`
- Receipt status: `ok`
- Bitterblossom run id: `ce498a5c64a1`

## Guardrail Read

- This slice uses deterministic key-recall evidence; no judge-only objective decides a winner.
- Full Crucible Harbor bundle digests are not present in this eval spec, so this is a transitional target import, not a final G2 approval packet.
- Search remains blocked until G1/G2 approval; the probe only establishes whether the target has headroom.
