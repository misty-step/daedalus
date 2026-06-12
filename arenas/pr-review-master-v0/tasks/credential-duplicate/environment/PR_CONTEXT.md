# Synthetic PR context

The PR adds token forwarding in `src/auth.py`. The intended behavior is to keep
the GitHub token only in process memory and never expose it in logs, argv, or
PR comments.
