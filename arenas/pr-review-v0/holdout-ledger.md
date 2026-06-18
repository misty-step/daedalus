# Holdout exposure ledger — pr-review-v0

Every `--final` scoring of holdout tasks is recorded here (`daedalus run`
appends automatically at stage 4). When a holdout task accumulates **5
exposure entries**, it is burned: rotate it into train/validation and author a
replacement (version bump).

| date | version | run | candidates exposed | tasks |
|---|---|---|---|---|
