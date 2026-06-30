#!/usr/bin/env python3
"""Parse cobertura.xml and produce a human-readable markdown coverage report."""

import xml.etree.ElementTree as ET
import json
import sys
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
    stats["percent"] = round(stats["lines_covered"] / stats["lines_valid"] * 100, 2) if stats["lines_valid"] else 0

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

            classes.append({
                "filename": filename,
                "rate": round(cls_rate * 100, 1),
                "lines_covered": len(hits_lines),
                "lines_total": len(hits_lines) + len(missed_lines),
                "hits_lines": hits_lines,
                "missed_lines": missed_lines,
            })

        packages.append({
            "name": pkg_name,
            "rate": round(pkg_rate * 100, 1),
            "classes": classes,
        })

    return {"stats": stats, "packages": packages}


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
    stats = data["stats"]
    packages = data["packages"]

    # Group by crate
    crates: dict[str, list] = {}
    for pkg in packages:
        for cls in pkg["classes"]:
            crate = extract_crate(cls["filename"])
            crates.setdefault(crate, []).append(cls)

    # Build markdown
    lines = []
    lines.append("# Coverage Report\n")
    lines.append(f"**{stats['lines_covered']}** / **{stats['lines_valid']}** lines covered — **{stats['percent']}%**\n")

    # Threshold check
    with open(BASE_DIR / ".coverage-baseline.json") as f:
        baseline = json.load(f)
    threshold = baseline.get("threshold", 80)
    status = "PASS" if stats["percent"] >= threshold else "FAIL"
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
            lines.append(f"- Coverage: {badge} **{pct}%** ({cls['lines_covered']}/{cls['lines_total']} lines)\n")

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

    # Write report
    markdown = "\n".join(lines)
    OUTPUT.write_text(markdown)
    print(f"Coverage report written to {OUTPUT}")
    print(f"  {stats['lines_covered']}/{stats['lines_valid']} lines ({stats['percent']}%)")
    print(f"  Status: {status}")


if __name__ == "__main__":
    main()
