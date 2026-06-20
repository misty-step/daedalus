# Ratio Zero Fixture

The change adds a guard that returns `0.0` for a zero denominator. A reviewer
should flag that this silently turns invalid input into a plausible ratio,
which can hide caller bugs.
