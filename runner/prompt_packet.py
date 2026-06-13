#!/usr/bin/env python3
"""Shared prompt-packet validation helpers for measured agent compositions."""


def _longest_run(text):
    longest = 0
    current = 0
    previous = None
    for char in text:
        if char == previous:
            current += 1
        else:
            previous = char
            current = 1
        longest = max(longest, current)
    return longest


def is_sane_prompt_packet(text):
    """Cheap syntactic guardrail against degenerate optimizer output.

    This is not a semantic quality judge; the arena still measures that. It
    only rejects packets that are too thin, absurdly large, or visibly
    corrupted, so a bad optimizer call cannot poison the seed or mutation
    pool with text like one repeated punctuation character.
    """
    if not isinstance(text, str):
        return False
    stripped = text.strip()
    if len(stripped) < 20 or len(stripped) > 4000:
        return False
    if _longest_run(stripped) > 24:
        return False
    visible = [char for char in stripped if not char.isspace()]
    if len(visible) >= 120:
        alpha_ratio = sum(char.isalpha() for char in visible) / len(visible)
        if alpha_ratio < 0.25:
            return False
        sample = visible[:500]
        unique_ratio = len(set(sample)) / len(sample)
        if unique_ratio < 0.05:
            return False
    return True
