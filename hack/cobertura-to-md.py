#!/usr/bin/env python3
"""Parse cobertura.xml and produce a human-readable markdown coverage report."""

import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
COBERTURA = BASE_DIR / "cobertura.xml"
OUTPUT = BASE_DIR / "COVERAGE_REPORT.md"


def parse_cobertura(path: Path) -> dict:
    tree = ET.parse(path)
    root = tree.getroot()

    stats = {
        "lines_covered": int(root.get("lines-covered", 0)),
        "lines_valid": int(root.get("lines-valid", 0)),
        "line_rate": float(root.get("line-rate", 0)),
    }
    if stats["lines_valid"]:
        stats["percent"] = round(stats["lines_covered"] / stats["lines_valid"] * 100, 2)
    else:
        stats["percent"] = 0

    packages = []
    for pkg in root.findall("packages/package"):
        pkg_name = pkg.get("name", "")
        pkg_rate = float(pkg.get("line-rate", 0))

        classes = []
        for cls in pkg.findall("classes/class"):
            filename = cls.get("filename", "")
            cls_rate = float(cls.get("line-rate", 0))
            lines_elem = cls.findall("lines/line")

            hits_lines = []
            missed_lines = []
            for ln in lines_elem:
                lineno = int(ln.get("number", 0))
                hits = int(ln.get("hits", 0))
                if hits > 0:
                    hits_lines.append(lineno)
                else:
                    missed_lines.append(lineno)

            classes.append(
                {
                    "filename": filename,
                    "rate": round(cls_rate * 100, 1),
                    "lines_covered": len(hits_lines),
                    "lines_total": len(hits_lines) + len(missed_lines),
                    "hits_lines": hits_lines,
                    "missed_lines": missed_lines,
                }
            )

        packages.append(
            {
                "name": pkg_name,
                "rate": round(pkg_rate * 100, 1),
                "classes": classes,
            }
        )

    return {"stats": stats, "packages": packages}


def is_main_rs(filepath: str) -> bool:
    """Return True if the file path ends with main.rs."""
    return Path(filepath).name == "main.rs"


def extract_crate(filepath: str) -> str:
    """Extract the crate name from the file path, e.g. 'crates/aetherd/src/...' -> 'aetherd'."""
    parts = Path(filepath).parts
    try:
        idx = parts.index("crates") + 1
        if idx < len(parts):
            return parts[idx]
    except (ValueError, IndexError):
        pass
    return "unknown"


def main():
    if not COBERTURA.exists():
        print(f"ERROR: {COBERTURA} not found. Run cargo tarpaulin first.", file=sys.stderr)
        sys.exit(1)

    data = parse_cobertura(COBERTURA)
    packages = data["packages"]

    # Filter out main.rs files before computing stats
    classes = []
    for pkg in packages:
        for cls in pkg["classes"]:
            if not is_main_rs(cls["filename"]):
                classes.append(cls)

    # Compute stats from filtered classes
    total_covered = sum(c["lines_covered"] for c in classes)
    total_valid = sum(c["lines_total"] for c in classes)
    percent = round(total_covered / total_valid * 100, 2) if total_valid else 0

    # Group by crate (from filtered classes)
    crates: dict[str, list] = {}
    for cls in classes:
        crate = extract_crate(cls["filename"])
        crates.setdefault(crate, []).append(cls)

    # Build markdown
    lines = []
    lines.append("# Coverage Report (excl. main.rs)\n")
    lines.append(f"**{total_covered}** / **{total_valid}** lines covered — **{percent}%**\n")

    # Threshold check
    with open(BASE_DIR / ".coverage-baseline.json") as f:
        baseline = json.load(f)
    threshold = baseline.get("threshold", 80)
    status = "PASS" if percent >= threshold else "FAIL"
    emoji = "✅" if status == "PASS" else "❌"
    lines.append(f"**Threshold:** {threshold}% | **Status:** {emoji} {status}\n")

    # Summary table
    lines.append("| Crate | Files | Covered | Total | Coverage |")
    lines.append("| --- | --- | --- | --- | --- |")
    total_covered = 0
    total_valid = 0
    for crate_name in sorted(crates):
        file_list = crates[crate_name]
        fc = sum(c["lines_covered"] for c in file_list)
        ft = sum(c["lines_total"] for c in file_list)
        pct = round(fc / ft * 100, 1) if ft else 0
        total_covered += fc
        total_valid += ft
        lines.append(f"| `{crate_name}` | {len(file_list)} | {fc} | {ft} | {pct}% |")
    lines.append("")

    # Per-crate detail
    for crate_name in sorted(crates):
        file_list = crates[crate_name]
        lines.append(f"## `{crate_name}`\n")

        for cls in sorted(file_list, key=lambda c: c["rate"]):
            pct = cls["rate"]
            # Color badge
            if pct >= 90:
                badge = "🟢"
            elif pct >= 70:
                badge = "🟡"
            else:
                badge = "🔴"

            lines.append(f"### {cls['filename']}\n")
            cov_text = f"- Coverage: {badge} **{pct}%**"
            cov_text += f" ({cls['lines_covered']}/{cls['lines_total']} lines)"
            lines.append(cov_text)

            if cls["missed_lines"]:
                lines.append("- **Missed lines:**")
                # Group consecutive missed lines
                groups = []
                start = cls["missed_lines"][0]
                end = cls["missed_lines"][0]
                for ln in cls["missed_lines"][1:]:
                    if ln == end + 1:
                        end = ln
                    else:
                        groups.append((start, end))
                        start = end = ln
                groups.append((start, end))

                for s, e in groups:
                    if s == e:
                        lines.append(f"  - Line {s}")
                    else:
                        lines.append(f"  - Lines {s}-{e}")
            lines.append("")

    # Write report only if content changed (to avoid pre-commit churn)
    markdown = "\n".join(lines)
    if OUTPUT.exists() and OUTPUT.read_text() == markdown:
        return  # No change
    OUTPUT.write_text(markdown)
    print(f"Coverage report written to {OUTPUT}")
    print(f"  {total_covered}/{total_valid} lines ({percent}%)")
    print(f"  Status: {status}")


if __name__ == "__main__":
    main()
