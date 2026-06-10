#!/usr/bin/env python3
"""Daedalus Phase 0 runner.

Executes one candidate composition against an arena of Harbor-format tasks,
scores each trial against the task's answer key, and appends run records as
JSONL under runs/. All interfaces are files (see DESIGN.md): candidate
manifests in candidates/, arenas in arenas/<id>/, run records in runs/.

Usage:
    runner/run.py --candidate candidates/oracle.toml --arena arenas/pr-review-v0
    runner/run.py --candidate candidates/pi-kimi.toml --arena arenas/pr-review-v0 \
        --tasks py-auth-sqli,js-cart-total --trials 3
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from score import score  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions"
RUNNER_VERSION = "0.1.0"


def utc_stamp():
    return datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def tree_digest(*roots):
    """Stable hash over file paths + contents, for grader tamper detection."""
    h = hashlib.sha256()
    for root in roots:
        for f in sorted(Path(root).rglob("*")):
            if f.is_file():
                h.update(str(f.relative_to(root)).encode())
                h.update(f.read_bytes())
    return h.hexdigest()


def validate_task_dir(task_dir):
    """Reject fixture trees that could leak paths outside the task directory."""
    for f in task_dir.rglob("*"):
        if f.is_symlink():
            raise RuntimeError(f"fixture contains symlink: {f}")


def task_instruction(arena_dir, arena, task_dir):
    """Compose the task instruction from the arena template + task intent,
    or fall back to a per-task instruction.md for template-less arenas."""
    template_ref = (arena.get("template") or {}).get("file")
    if template_ref:
        template = (arena_dir / template_ref).read_text()
        intent = (task_dir / "intent.md").read_text().strip()
        return template.replace("{intent}", intent)
    return (task_dir / "instruction.md").read_text()


def select_tasks(arena_dir, arena, split, task_filter, final):
    """Resolve task dirs honoring split selection and the holdout guard:
    holdout fixtures are scored only behind --final, so the search loop can
    never overfit them."""
    split_cfg = arena.get("split") or {}
    if split != "all":
        ids = split_cfg.get(split)
        if not ids:
            sys.exit(f"arena declares no '{split}' split")
        allowed = set(ids)
    else:
        allowed = None
    task_dirs = [
        d
        for d in sorted((arena_dir / "tasks").iterdir())
        if d.is_dir()
        and (allowed is None or d.name in allowed)
        and (task_filter is None or d.name in task_filter)
    ]
    holdout = set(split_cfg.get("holdout", []))
    touched_holdout = sorted(d.name for d in task_dirs if d.name in holdout)
    if touched_holdout and not final:
        sys.exit(
            "holdout tasks require --final (anti-overfitting guard): "
            + ", ".join(touched_holdout)
        )
    return task_dirs


def load_toml(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def load_candidate(path):
    """Load a manifest, resolve its prompt packet, compute the composition
    hash. Private keys (underscore-prefixed) never reach disk records."""
    candidate = load_toml(path)
    packet_ref = candidate.get("prompt_packet")
    if packet_ref:
        packet_path = Path(packet_ref)
        if not packet_path.is_absolute():
            packet_path = REPO / packet_path
        candidate["_packet_text"] = packet_path.read_text()
    else:
        candidate["_packet_text"] = None
    basis = {k: v for k, v in candidate.items() if not k.startswith("_")}
    basis["prompt_packet_text"] = candidate["_packet_text"]
    candidate["_hash"] = hashlib.sha256(
        json.dumps(basis, sort_keys=True).encode()
    ).hexdigest()[:16]
    return candidate


def harness_version(candidate):
    if candidate["kind"] != "pi":
        return None
    try:
        out = subprocess.run(
            ["pi", "--version"], capture_output=True, text=True, timeout=30
        )
        return out.stdout.strip() or None
    except Exception:  # noqa: BLE001 - version capture is best-effort
        return None


def workspace_listing(workdir):
    parts = []
    for f in sorted(workdir.rglob("*")):
        if f.is_file() and f.name != "findings.json":
            rel = f.relative_to(workdir)
            parts.append(f"\n### {rel}\n```\n{f.read_text()}```\n")
    return "".join(parts)


def extract_json_object(text):
    """Pull the first parseable top-level JSON object out of model output."""
    start = text.find("{")
    while start != -1:
        depth = 0
        for i in range(start, len(text)):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    try:
                        return json.loads(text[start : i + 1])
                    except json.JSONDecodeError:
                        break
        start = text.find("{", start + 1)
    raise ValueError("no JSON object found in model output")


def run_null(candidate, instruction, task_dir, workdir, record):
    (workdir / "findings.json").write_text('{"findings": []}\n')


def run_oracle(candidate, instruction, task_dir, workdir, record):
    shutil.copy(task_dir / "solution" / "findings.json", workdir / "findings.json")


def run_oneshot(candidate, instruction, task_dir, workdir, record):
    key = os.environ.get("OPENROUTER_API_KEY")
    if not key:
        raise RuntimeError("OPENROUTER_API_KEY is not set")
    prompt = (
        instruction
        + "\n\n## Workspace files\n"
        + workspace_listing(workdir)
        + "\nRespond with ONLY the findings JSON object, no prose."
    )
    system = (
        candidate["_packet_text"]
        or "You are a precise code-review agent. Output only valid JSON."
    )
    body = {
        "model": candidate["model"],
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": prompt},
        ],
        "temperature": candidate.get("temperature", 0.2),
        "max_tokens": candidate.get("max_tokens", 8192),
        "usage": {"include": True},
    }
    if "provider" in candidate:
        body["provider"] = candidate["provider"]
    req = urllib.request.Request(
        OPENROUTER_URL,
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=candidate.get("timeout_sec", 300)) as resp:
        payload = json.loads(resp.read())
    usage = payload.get("usage") or {}
    record["provider_served"] = payload.get("provider")
    record["tokens_prompt"] = usage.get("prompt_tokens")
    record["tokens_completion"] = usage.get("completion_tokens")
    record["tokens_cached"] = (usage.get("prompt_tokens_details") or {}).get(
        "cached_tokens"
    )
    record["cost_usd"] = usage.get("cost")
    content = payload["choices"][0]["message"]["content"]
    record["_response_text"] = content
    findings = extract_json_object(content)
    (workdir / "findings.json").write_text(json.dumps(findings, indent=2))


# Baseline process environment for candidate subprocesses. Everything else —
# notably every API key except the candidate's own allowlist — is withheld.
BASE_ENV_VARS = ("PATH", "HOME", "TERM", "LANG", "LC_ALL")


def candidate_env(candidate):
    allow = candidate.get("env_allowlist", ["OPENROUTER_API_KEY"])
    return {
        k: os.environ[k]
        for k in (*BASE_ENV_VARS, *allow)
        if k in os.environ
    }


def extract_pi_usage(stdout_text):
    """Sum usage across assistant message_end events in pi --mode json output."""
    tokens_in = tokens_out = cached = 0
    cost = 0.0
    provider = None
    found = False
    for line in stdout_text.splitlines():
        line = line.strip()
        if not line.startswith('{"type":"message_end"'):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        msg = event.get("message") or {}
        if msg.get("role") != "assistant":
            continue
        usage = msg.get("usage") or {}
        found = True
        provider = msg.get("provider") or provider
        tokens_in += int(usage.get("input") or 0)
        tokens_out += int(usage.get("output") or 0)
        cached += int(usage.get("cacheRead") or 0)
        cost += float((usage.get("cost") or {}).get("total") or 0)
    if not found:
        return {}
    return {
        "provider_served": provider,
        "tokens_prompt": tokens_in,
        "tokens_completion": tokens_out,
        "tokens_cached": cached,
        "cost_usd": round(cost, 6),
    }


def run_pi(candidate, instruction, task_dir, workdir, record):
    cmd = [
        "pi",
        "-p",
        "--mode",
        "json",
        "--no-session",
        "--no-extensions",
        "--no-skills",
        "--no-prompt-templates",
        "--no-themes",
        "--no-context-files",
        "--provider",
        candidate.get("provider_name", "openrouter"),
        "--model",
        candidate["model"],
    ]
    if "thinking" in candidate:
        cmd += ["--thinking", candidate["thinking"]]
    if "tools" in candidate:
        cmd += ["--tools", ",".join(candidate["tools"])]
    if candidate["_packet_text"]:
        cmd += ["--append-system-prompt", candidate["_packet_text"]]
    message = instruction + "\n\nThe workspace is the current working directory."
    proc = subprocess.run(
        cmd + [message],
        cwd=workdir,
        capture_output=True,
        text=True,
        timeout=candidate.get("timeout_sec", 600),
        env=candidate_env(candidate),
    )
    record["agent_exit_code"] = proc.returncode
    record["_transcript_text"] = proc.stdout
    record.update(extract_pi_usage(proc.stdout))
    if proc.returncode != 0:
        # A crashing candidate is a failed trial even if it left findings;
        # silent partial credit would let broken runs look valid.
        raise RuntimeError(f"pi exited {proc.returncode}: {proc.stderr[-400:]}")


EXECUTORS = {
    "null": run_null,
    "oracle": run_oracle,
    "oneshot": run_oneshot,
    "pi": run_pi,
}


def summarize(trials_path):
    """Per-candidate, per-task reward distributions from a trials file."""
    records = [json.loads(line) for line in trials_path.read_text().splitlines()]
    summary = {}
    for r in records:
        c = summary.setdefault(
            r["candidate_id"],
            {
                "composition_hash": r.get("composition_hash"),
                "tasks": {},
                "trials": 0,
                "errors": 0,
                "cost_usd_total": 0.0,
                "cost_known": True,
            },
        )
        c["trials"] += 1
        if r.get("error"):
            c["errors"] += 1
        t = c["tasks"].setdefault(r["task_id"], {"rewards": [], "wall_ms": []})
        t["rewards"].append(r["reward"])
        t["wall_ms"].append(r["wall_ms"])
        if r.get("cost_usd") is None:
            c["cost_known"] = False
        else:
            c["cost_usd_total"] += r["cost_usd"]
    for c in summary.values():
        rewards = [x for t in c["tasks"].values() for x in t["rewards"]]
        c["reward_mean"] = round(sum(rewards) / len(rewards), 4)
        c["cost_usd_total"] = round(c["cost_usd_total"], 6)
        for t in c["tasks"].values():
            t["mean"] = round(sum(t["rewards"]) / len(t["rewards"]), 4)
            t["min"] = min(t["rewards"])
            t["max"] = max(t["rewards"])
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, help="candidate manifest TOML")
    parser.add_argument("--arena", required=True, help="arena directory")
    parser.add_argument("--tasks", help="comma-separated task ids (default: all)")
    parser.add_argument("--trials", type=int, default=1)
    parser.add_argument(
        "--exp-dir",
        help="append into an existing experiment directory (loop driver mode)",
    )
    parser.add_argument(
        "--split",
        choices=["train", "validation", "holdout", "all"],
        default="all",
        help="restrict to a declared arena split (default: all)",
    )
    parser.add_argument(
        "--final",
        action="store_true",
        help="allow scoring holdout tasks (final evaluation only)",
    )
    args = parser.parse_args()

    candidate = load_candidate(args.candidate)
    arena_dir = Path(args.arena)
    arena = load_toml(arena_dir / "arena.toml")
    run_fn = EXECUTORS[candidate["kind"]]

    task_filter = set(args.tasks.split(",")) if args.tasks else None
    task_dirs = select_tasks(arena_dir, arena, args.split, task_filter, args.final)
    if not task_dirs:
        sys.exit("no tasks matched")

    stamp = utc_stamp()
    runs_root = Path(os.environ.get("DAEDALUS_RUNS_DIR", REPO / "runs"))
    exp_dir = Path(args.exp_dir) if args.exp_dir else (
        runs_root / f"{stamp}-{candidate['id']}"
    )
    (exp_dir / "compositions").mkdir(parents=True, exist_ok=True)

    snapshot = {k: v for k, v in candidate.items() if not k.startswith("_")}
    snapshot.update(
        composition_hash=candidate["_hash"],
        prompt_packet_text=candidate["_packet_text"],
        harness_version=harness_version(candidate),
        runner_version=RUNNER_VERSION,
    )
    (exp_dir / "compositions" / f"{candidate['id']}.json").write_text(
        json.dumps(snapshot, indent=2)
    )
    trials_path = exp_dir / "trials.jsonl"

    records = []
    for task_dir in task_dirs:
        validate_task_dir(task_dir)
        instruction = task_instruction(arena_dir, arena, task_dir)
        grader_digest = tree_digest(task_dir / "tests", task_dir / "solution")
        for trial in range(1, args.trials + 1):
            record = {
                "run_id": f"{stamp}-{candidate['id']}-{task_dir.name}-t{trial}",
                "ts_start": datetime.now(timezone.utc).isoformat(timespec="seconds"),
                "runner_version": RUNNER_VERSION,
                "arena_id": arena["id"],
                "arena_version": arena["version"],
                "taskspec": arena.get("taskspec"),
                "task_id": task_dir.name,
                "trial": trial,
                "candidate_id": candidate["id"],
                "candidate_kind": candidate["kind"],
                "composition_hash": candidate["_hash"],
                "harness_version": snapshot["harness_version"],
                "model": candidate.get("model"),
                "provider_served": None,
                "tokens_prompt": None,
                "tokens_completion": None,
                "tokens_cached": None,
                "cost_usd": None,
                "error": None,
            }
            workdir = Path(tempfile.mkdtemp(prefix=f"daedalus-{task_dir.name}-"))
            t0 = time.monotonic()
            try:
                shutil.copytree(task_dir / "environment", workdir, dirs_exist_ok=True)
                run_fn(candidate, instruction, task_dir, workdir, record)
            except Exception as exc:  # noqa: BLE001 - record and keep going
                record["error"] = str(exc)
            record["wall_ms"] = int((time.monotonic() - t0) * 1000)
            record["ts_end"] = datetime.now(timezone.utc).isoformat(timespec="seconds")

            if tree_digest(task_dir / "tests", task_dir / "solution") != grader_digest:
                record["error"] = "grader files modified during run; trial voided"
            if record["error"]:
                # Failed or compromised trials never earn reward.
                record.update(
                    reward=0.0,
                    recall=0.0,
                    matched=[],
                    false_positives=0,
                    expected_defects=None,
                    scorer_error=None,
                )
            else:
                verdict = score(
                    workdir / "findings.json", task_dir / "tests" / "expected.json"
                )
                record.update(
                    reward=verdict["reward"],
                    recall=verdict["recall"],
                    matched=verdict["matched"],
                    false_positives=verdict["false_positives"],
                    expected_defects=verdict["expected_defects"],
                    scorer_error=verdict["error"],
                )
            findings_file = workdir / "findings.json"
            if findings_file.exists():
                try:
                    record["findings"] = json.loads(findings_file.read_text()).get(
                        "findings"
                    )
                except json.JSONDecodeError:
                    record["findings"] = None

            # Retain evidence; artifacts/ is gitignored, records are not.
            art_dir = (
                exp_dir / "artifacts" / candidate["id"]
                / f"{task_dir.name}-t{trial}-{stamp}"
            )
            art_dir.mkdir(parents=True, exist_ok=True)
            transcript = record.pop("_transcript_text", None)
            if transcript:
                (art_dir / "transcript.txt").write_text(transcript)
            response = record.pop("_response_text", None)
            if response:
                (art_dir / "response.txt").write_text(response)
            if findings_file.exists():
                shutil.copy(findings_file, art_dir / "findings.json")
            record["artifacts"] = str(art_dir.relative_to(exp_dir))
            shutil.rmtree(workdir, ignore_errors=True)

            with open(trials_path, "a") as f:
                f.write(json.dumps(record) + "\n")
            records.append(record)
            cost = record["cost_usd"]
            print(
                f"{task_dir.name:<18} t{trial}  reward={record['reward']:<6}"
                f" wall={record['wall_ms']/1000:.1f}s"
                f" cost={'$%.4f' % cost if cost is not None else 'unknown'}"
                f"{'  ERROR: ' + record['error'] if record['error'] else ''}",
                flush=True,
            )

    summary = summarize(trials_path)
    (exp_dir / "summary.json").write_text(json.dumps(summary, indent=2))
    rewards = [r["reward"] for r in records]
    costs = [r["cost_usd"] for r in records if r["cost_usd"] is not None]
    print(f"\ncandidate: {candidate['id']}  ({len(records)} trials)", flush=True)
    print(f"mean reward: {sum(rewards)/len(rewards):.4f}", flush=True)
    print(
        f"total cost:  ${sum(costs):.4f}" if costs else "total cost:  unknown",
        flush=True,
    )
    shown = exp_dir.relative_to(REPO) if exp_dir.is_relative_to(REPO) else exp_dir
    print(f"experiment:  {shown}", flush=True)


if __name__ == "__main__":
    main()
