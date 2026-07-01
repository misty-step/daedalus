# Vendored report assets

These CSS files are embedded into the `threshold` binary (`include_str!` in
`src/report_html/render.rs`) and inlined into every `report-html` document so the
report is self-contained and offline (`file://`, PR-attachable, air-gap-safe).
They are byte-identical snapshots of their upstream sources — do not edit here;
edit upstream and re-copy so the diff stays auditable.

| file            | source                                              | version        |
|-----------------|-----------------------------------------------------|----------------|
| `aesthetic.css` | `~/Development/aesthetic/aesthetic.css`              | v2.6.0         |
| `lab.css`       | `docs/threshold-ui-lab/round-2/lab.css` (this repo)   | round-2 lab    |

## Re-sync

```sh
cp ~/Development/aesthetic/aesthetic.css crates/threshold-core/assets/aesthetic.css
cp docs/threshold-ui-lab/round-2/lab.css  crates/threshold-core/assets/lab.css
cargo test -p threshold-core report_html::tests::renders_a_self_contained_document -- --exact
```

The `renders_a_self_contained_document` test fails if a vendored file gains a
network reference (`@import url(http…)`, web-font `url(http…)`), which would
break the offline guarantee — so drift that matters is caught mechanically.
