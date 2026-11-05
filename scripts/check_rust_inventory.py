#!/usr/bin/env python3
"""Check name coverage of the SCARS -> Rust matrix; not a behavior verifier."""
from pathlib import Path
import re

root = Path(__file__).resolve().parent.parent
source = (root / "scars/src/scars.cppm").read_text().split("export namespace scars {", 1)[1]
matrix = (root / "docs/rust-inventory.md").read_text()
# Remove comments/string literals before counting braces; all exported top-level
# types live directly inside scars, whereas implementation types live in detail.
clean = re.sub(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'',
               lambda match: "\n" * match[0].count("\n"), source, flags=re.S)
names, depth = set(), 0
for line in clean.splitlines():
    if depth == 0:
        match = re.match(r"(?:template\s*<.*?>\s*)?(?:enum class|struct|class|using)\s+(\w+)", line)
        if match:
            names.add(match[1])
    depth += line.count("{") - line.count("}")
    if depth < 0:
        break  # End of the exported namespace; later detail contracts are private.
# View methods are intentionally checked separately from opaque model APIs.
view = source.split("class View {", 1)[1].split("  private:", 1)[0]
modifiers = set(re.findall(r"\bView\s+(\w+)\(", view)) | {"description", "labeled"}
covered = set(re.findall(r"`([^`]+)`", matrix))
missing = (names | modifiers) - covered
if missing:
    raise SystemExit("Unmapped public names: " + ", ".join(sorted(missing)))
print(f"Mapped {len(names)} exported types and {len(modifiers)} view/annotation names.")
print("Behavioral completion is tracked separately in docs/rust-completion-checklist.md.")
