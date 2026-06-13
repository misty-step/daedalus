from pathlib import Path
import re
import tomllib


REPO = Path(__file__).resolve().parent.parent


def _contract():
    text = (REPO / "docs/review-autoresearch-loop.md").read_text()
    match = re.search(r"```toml\n(.*?)\n```", text, re.DOTALL)
    assert match, "autoresearch loop doc must expose a TOML contract"
    return tomllib.loads(match.group(1))


def test_review_autoresearch_loop_contract_is_scoped_to_next_lane():
    contract = _contract()

    assert contract["schema"] == "review-autoresearch-loop.v1"
    assert contract["program"] == "pr-review-swarm"
    assert contract["first_lane"] == "correctness"
    assert contract["next_arena_iteration"] == "pr-review-correctness-v0.2"
    assert contract["sandbox_boundary"] == "member-artifacts-only-before-g3"
    assert contract["do_not_average_across_arena_versions"] is True
    assert contract["full_swarm_blocked_until"] == [
        "correctness-v0.2",
        "real-member-replay",
    ]
    assert contract["required_loop_evidence"] == [
        "primitive-refresh",
        "arena-freeze",
        "controlled-hypothesis",
        "certified-search-or-postmortem",
    ]


def test_review_swarm_backlog_points_at_autoresearch_loop():
    text = (REPO / "backlog.d/034-build-daedalus-review-swarm.md").read_text()

    assert "docs/review-autoresearch-loop.md" in text
    assert "correctness v0.2 autoresearch loop" in text
    assert "runtime-crash" in text
