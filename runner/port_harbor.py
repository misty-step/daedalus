#!/usr/bin/env python3
"""Render a Daedalus arena into a Harbor-format build directory.

    runner/port_harbor.py arenas/pr-review-v0 [--out harbor-build]

Output per task: instruction.md (template+intent rendered), Harbor task.toml,
environment/Dockerfile + workspace files, tests/ (self-contained verifier:
test.sh + score.py + expected.json), solution/ (solve.sh + findings.json).
The build dir is a generated artifact (gitignored); the arena stays the
single source of truth.
"""

import argparse
import shutil
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

DOCKERFILE = """\
FROM python:3.12-slim
WORKDIR /app
COPY . /app/
"""

DOCKERIGNORE = "Dockerfile\n.dockerignore\n"

TEST_SH = """\
#!/bin/bash
# Daedalus verifier: score findings.json against the answer key and emit the
# Harbor reward file.
set -u
python3 /tests/score.py /app/findings.json /tests/expected.json \\
    > /logs/verifier/score.json
python3 - <<'EOF' > /logs/verifier/reward.txt
import json
print(json.load(open("/logs/verifier/score.json"))["reward"])
EOF
"""

SOLVE_SH = """\
#!/bin/bash
# Oracle: replay the reference findings.
cp /solution/findings.json /app/findings.json
"""

TASK_TOML = """\
version = "1.0"

[agent]
timeout_sec = {agent_timeout}

[verifier]
timeout_sec = {verifier_timeout}
"""


def render_instruction(arena_dir, arena, task_dir):
    template = (arena_dir / arena["template"]["file"]).read_text()
    intent = (task_dir / "intent.md").read_text().strip()
    return template.replace("{intent}", intent)


def port_task(arena_dir, arena, task_dir, out_dir):
    src_cfg = tomllib.loads((task_dir / "task.toml").read_text())
    out_dir.mkdir(parents=True, exist_ok=True)

    (out_dir / "instruction.md").write_text(
        render_instruction(arena_dir, arena, task_dir)
    )
    (out_dir / "task.toml").write_text(
        TASK_TOML.format(
            agent_timeout=float(src_cfg.get("agent", {}).get("timeout_sec", 600)),
            verifier_timeout=float(
                src_cfg.get("verifier", {}).get("timeout_sec", 120)
            ),
        )
    )

    env_out = out_dir / "environment"
    if env_out.exists():
        shutil.rmtree(env_out)
    shutil.copytree(task_dir / "environment", env_out)
    (env_out / "Dockerfile").write_text(DOCKERFILE)
    (env_out / ".dockerignore").write_text(DOCKERIGNORE)

    tests_out = out_dir / "tests"
    if tests_out.exists():
        shutil.rmtree(tests_out)
    tests_out.mkdir()
    shutil.copy(task_dir / "tests" / "expected.json", tests_out / "expected.json")
    shutil.copy(REPO / "runner" / "score.py", tests_out / "score.py")
    test_sh = tests_out / "test.sh"
    test_sh.write_text(TEST_SH)
    test_sh.chmod(0o755)

    sol_out = out_dir / "solution"
    if sol_out.exists():
        shutil.rmtree(sol_out)
    sol_out.mkdir()
    shutil.copy(task_dir / "solution" / "findings.json", sol_out / "findings.json")
    solve_sh = sol_out / "solve.sh"
    solve_sh.write_text(SOLVE_SH)
    solve_sh.chmod(0o755)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("arena", help="arena directory")
    parser.add_argument("--out", default="harbor-build")
    args = parser.parse_args()

    arena_dir = Path(args.arena)
    arena = tomllib.loads((arena_dir / "arena.toml").read_text())
    build_root = REPO / args.out / arena["id"]

    task_dirs = sorted(d for d in (arena_dir / "tasks").iterdir() if d.is_dir())
    if not task_dirs:
        sys.exit("no tasks found")
    for task_dir in task_dirs:
        port_task(arena_dir, arena, task_dir, build_root / task_dir.name)
        print(f"ported {task_dir.name}", flush=True)
    print(f"harbor dataset: {build_root}", flush=True)


if __name__ == "__main__":
    main()
