"""Review-swarm taxonomy validation.

The taxonomy is a human-readable Markdown document with one fenced TOML block
as the machine contract. The suite spec names the required/optional members and
thresholds. This validator checks that the two agree before any arena fixtures
are authored.
"""

import re
import tomllib
from dataclasses import dataclass, field
from pathlib import Path


class TaxonomyError(RuntimeError):
    """Raised when the taxonomy document cannot be parsed."""


@dataclass
class TaxonomyReport:
    ok: bool = True
    messages: list[str] = field(default_factory=list)
    lenses: list[str] = field(default_factory=list)
    categories: list[str] = field(default_factory=list)

    def fail(self, message):
        self.ok = False
        self.messages.append(message)


def _load_toml(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def _taxonomy_block(path):
    text = Path(path).read_text()
    matches = re.findall(r"```toml\n(.*?)\n```", text, flags=re.DOTALL)
    for block in matches:
        if 'schema = "review-swarm-taxonomy.v1"' in block:
            return tomllib.loads(block)
    raise TaxonomyError(
        f"{path}: missing fenced TOML block with schema review-swarm-taxonomy.v1"
    )


def _require_list(data, key, report, label):
    value = data.get(key)
    if not isinstance(value, list) or not value or not all(
        isinstance(item, str) and item for item in value
    ):
        report.fail(f"{label} must be a non-empty string list")
        return []
    return value


def _require_thresholds(suite, report):
    suite_table = suite.get("suite") or {}
    for key in ("cost_ceiling_usd", "wall_ceiling_sec"):
        value = suite_table.get(key)
        if not isinstance(value, (int, float)):
            report.fail(f"suite.{key} must be numeric")
    thresholds = suite.get("suite", {}).get("thresholds") or {}
    required = [
        "master_recall_min",
        "blocking_recall_min",
        "false_positive_carry_max",
        "duplicate_collapse_min",
    ]
    for key in required:
        if key not in thresholds:
            report.fail(f"suite.thresholds missing {key}")


def _validate_member_artifact(suite, taxonomy_data, report):
    member = suite.get("member_artifact") or {}
    if member.get("schema") != "review-swarm-member-artifact.v1":
        report.fail("member_artifact.schema must be review-swarm-member-artifact.v1")
    severity_levels = (taxonomy_data.get("severity") or {}).get("levels") or []
    if member.get("severities") != severity_levels:
        report.fail("member_artifact.severities must match taxonomy severity levels")
    for key in ("statuses", "confidences"):
        value = member.get(key)
        if not isinstance(value, list) or not value:
            report.fail(f"member_artifact.{key} must be a non-empty list")


def _repo_root_for_paths(suite_path):
    suite_path = Path(suite_path).resolve()
    for parent in (suite_path.parent, *suite_path.parents):
        if (parent / "AGENTS.md").exists() and (parent / "runner").is_dir():
            return parent
    return Path.cwd()


def _scaffold_only(spec):
    return (spec.get("scaffold") or {}).get("runnable") is False


def _validate_scaffold(spec, report, label):
    if not _scaffold_only(spec):
        return False
    scaffold = spec.get("scaffold") or {}
    if "search" in spec:
        report.fail(f"{label} is scaffold-only and must not declare [search]")
    if not isinstance(scaffold.get("blocked_on"), str) or not scaffold.get("blocked_on"):
        report.fail(f"{label}.scaffold.blocked_on must be non-empty")
    return True


def _validate_base_packet(spec, base, report, label):
    if _scaffold_only(spec):
        return
    packet = (spec.get("search") or {}).get("base_packet")
    if not isinstance(packet, str) or not packet:
        report.fail(f"{label}.search.base_packet must be a non-empty path")
    elif not (base / packet).exists():
        report.fail(f"{label}.search.base_packet does not exist: {packet}")


def _validate_lens_adapter(spec, base, report, label):
    lens = spec.get("lens") or {}
    if not lens:
        return
    if _scaffold_only(spec):
        if not isinstance(lens.get("blocked_on"), str) or not lens.get("blocked_on"):
            report.fail(f"{label}.lens.blocked_on must be non-empty")
        return
    adapted_from = lens.get("adapted_from")
    if not isinstance(adapted_from, str) or not adapted_from:
        report.fail(f"{label}.lens.adapted_from must be a non-empty path")
        return
    arena = base / adapted_from
    if not arena.is_dir():
        report.fail(f"{label}.lens.adapted_from does not exist: {adapted_from}")
        return
    tasks = lens.get("adapted_tasks") or []
    if not isinstance(tasks, list) or not tasks:
        report.fail(f"{label}.lens.adapted_tasks must be a non-empty list")
        return
    for task in tasks:
        if not isinstance(task, str) or not task:
            report.fail(f"{label}.lens.adapted_tasks contains a non-string task")
        elif not (arena / "tasks" / task).is_dir():
            report.fail(f"{label}.lens.adapted_tasks missing task: {task}")


def _validate_suite_paths(suite, suite_path, required_members, optional_members, report):
    base = _repo_root_for_paths(suite_path)
    _validate_base_packet(suite, base, report, "suite")
    suite_table = suite.get("suite") or {}
    master_spec = suite_table.get("master_spec")
    if not isinstance(master_spec, str) or not master_spec:
        report.fail("suite.master_spec must be a non-empty path")
    elif not (base / master_spec).exists():
        report.fail(f"suite.master_spec does not exist: {master_spec}")

    member_tables = suite_table.get("members") or {}
    if not isinstance(member_tables, dict):
        report.fail("suite.members must be a table")
        member_tables = {}
    for member in required_members:
        if member not in member_tables:
            report.fail(f"suite.members missing required member: {member}")
    allowed = set(required_members) | set(optional_members)
    for member, table in member_tables.items():
        if member not in allowed:
            report.fail(f"suite.members contains unknown member: {member}")
            continue
        if not isinstance(table, dict):
            report.fail(f"suite.members.{member} must be a table")
            continue
        for key in ("spec", "role", "status", "evidence"):
            if not isinstance(table.get(key), str) or not table.get(key):
                report.fail(f"suite.members.{member}.{key} must be non-empty")
        for key in ("spec", "evidence"):
            ref = table.get(key)
            if isinstance(ref, str) and ref and not (base / ref).exists():
                report.fail(f"suite.members.{member}.{key} does not exist: {ref}")
        spec_ref = table.get("spec")
        if isinstance(spec_ref, str) and (base / spec_ref).exists():
            try:
                member_spec = _load_toml(base / spec_ref)
            except tomllib.TOMLDecodeError as exc:
                report.fail(f"suite.members.{member}.spec is invalid TOML: {exc}")
                continue
            _validate_scaffold(
                member_spec,
                report,
                f"suite.members.{member}.spec",
            )
            _validate_base_packet(
                member_spec,
                base,
                report,
                f"suite.members.{member}.spec",
            )
            _validate_lens_adapter(
                member_spec,
                base,
                report,
                f"suite.members.{member}.spec",
            )


def validate_taxonomy(taxonomy_path, suite_path):
    """Return a validation report for taxonomy doc + suite taskspec."""
    report = TaxonomyReport()
    try:
        taxonomy_data = _taxonomy_block(taxonomy_path)
    except (OSError, TaxonomyError, tomllib.TOMLDecodeError) as exc:
        report.fail(str(exc))
        return report
    try:
        suite = _load_toml(suite_path)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        report.fail(f"{suite_path}: invalid suite spec: {exc}")
        return report

    if taxonomy_data.get("schema") != "review-swarm-taxonomy.v1":
        report.fail("taxonomy schema must be review-swarm-taxonomy.v1")
    lenses = _require_list(taxonomy_data, "lenses", report, "lenses")
    required_lenses = _require_list(
        taxonomy_data, "required_lenses", report, "required_lenses"
    )
    optional_lenses = _require_list(
        taxonomy_data, "optional_lenses", report, "optional_lenses"
    )
    report.lenses = lenses
    lens_set = set(lenses)
    for lens in required_lenses + optional_lenses:
        if lens not in lens_set:
            report.fail(f"declared lens not present in lenses: {lens}")

    suite_table = suite.get("suite") or {}
    required_members = suite_table.get("required_members") or []
    optional_members = suite_table.get("optional_members") or []
    for member in required_members:
        if member not in lens_set:
            report.fail(f"required member missing from taxonomy lenses: {member}")
    for member in optional_members:
        if member not in lens_set:
            report.fail(f"optional member missing from taxonomy lenses: {member}")
    overlap = set(required_members) & set(optional_members)
    if overlap:
        report.fail(
            "members cannot be both required and optional: "
            + ", ".join(sorted(overlap))
        )

    severity = taxonomy_data.get("severity") or {}
    levels = severity.get("levels") or []
    if levels != ["blocking", "serious", "minor"]:
        report.fail("severity.levels must be blocking, serious, minor")
    if not severity.get("blocking_rule"):
        report.fail("severity.blocking_rule must not be empty")

    categories = taxonomy_data.get("category") or []
    if not isinstance(categories, list) or not categories:
        report.fail("at least one [[category]] is required")
        categories = []
    seen_categories = set()
    categories_by_lens = {lens: 0 for lens in lenses}
    for category in categories:
        cid = category.get("id")
        lens = category.get("lens")
        if not isinstance(cid, str) or not cid:
            report.fail("category missing id")
            continue
        if cid in seen_categories:
            report.fail(f"duplicate category id: {cid}")
        seen_categories.add(cid)
        report.categories.append(cid)
        if lens not in lens_set:
            report.fail(f"category {cid} uses unknown lens: {lens}")
        else:
            categories_by_lens[lens] += 1
        for key in ("description", "blocking_rule"):
            if not isinstance(category.get(key), str) or not category.get(key):
                report.fail(f"category {cid} missing {key}")
        overlaps = category.get("allowed_overlaps") or []
        if not isinstance(overlaps, list):
            report.fail(f"category {cid} allowed_overlaps must be a list")
            overlaps = []
        for item in overlaps:
            if item not in lens_set:
                report.fail(f"category {cid} uses unknown overlap lens: {item}")
    for lens in required_members:
        if categories_by_lens.get(lens, 0) == 0:
            report.fail(f"required member has no taxonomy category: {lens}")

    overlaps = taxonomy_data.get("overlap") or []
    if not isinstance(overlaps, list):
        report.fail("[[overlap]] entries must be a list")
        overlaps = []
    seen_overlaps = set()
    for item in overlaps:
        oid = item.get("id")
        if not isinstance(oid, str) or not oid:
            report.fail("overlap missing id")
            continue
        if oid in seen_overlaps:
            report.fail(f"duplicate overlap id: {oid}")
        seen_overlaps.add(oid)
        owner = item.get("owner")
        if owner not in lens_set:
            report.fail(f"overlap {oid} owner uses unknown lens: {owner}")
        overlap_lenses = item.get("lenses") or []
        if not isinstance(overlap_lenses, list) or len(overlap_lenses) < 2:
            report.fail(f"overlap {oid} must name at least two lenses")
            overlap_lenses = []
        for lens in overlap_lenses:
            if lens not in lens_set:
                report.fail(f"overlap {oid} uses unknown lens: {lens}")
        if owner and owner not in overlap_lenses:
            report.fail(f"overlap {oid} owner must be one of its lenses")
        if not item.get("rule"):
            report.fail(f"overlap {oid} missing rule")

    _require_thresholds(suite, report)
    _validate_member_artifact(suite, taxonomy_data, report)
    _validate_suite_paths(suite, suite_path, required_members, optional_members, report)
    return report


def render_report(report):
    status = "PASS" if report.ok else "FAIL"
    lines = [
        f"Taxonomy validation: {status}",
        f"lenses: {', '.join(report.lenses) if report.lenses else '-'}",
        f"categories: {', '.join(report.categories) if report.categories else '-'}",
    ]
    if report.messages:
        lines.append("findings:")
        lines.extend(f"- {message}" for message in report.messages)
    return "\n".join(lines)
