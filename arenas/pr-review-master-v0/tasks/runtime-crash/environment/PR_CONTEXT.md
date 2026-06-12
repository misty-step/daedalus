# Synthetic PR context

The PR changes request parsing in `src/ingest.py`. A missing `payload` field is
valid input and should return a structured validation error, not crash.
