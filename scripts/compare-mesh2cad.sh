#!/usr/bin/env bash
# Sets meshwright's parameters beside mesh2cad's for each STL file, as a Markdown table.
#
#   scripts/compare-mesh2cad.sh plugin/meshwright/tests/*.stl
#
# mesh2cad must be installed with CadQuery (`pip install "mesh2cad[cadquery]"`); set MESH2CAD to
# its executable if it is not on PATH. Each row is one parameter name. A name only one tool
# writes shows "—" for the other; a value that differs by more than 0.001 is marked with "≠".
# mesh2cad's verdict (the script it wrote, or why it refused) and meshwright's import summary
# follow each table. See docs/development.md.
set -euo pipefail

MESH2CAD=${MESH2CAD:-mesh2cad}
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

if [ $# -eq 0 ]; then
  echo "usage: $0 <file.stl>..." >&2
  exit 2
fi
command -v "$MESH2CAD" >/dev/null || { echo "mesh2cad not found; set MESH2CAD" >&2; exit 2; }
cargo build -q --manifest-path "$root/Cargo.toml" -p meshwright-core --example params

for stl in "$@"; do
  base=$(basename "$stl" .stl)
  echo "### $base"
  echo
  m2c_verdict=$("$MESH2CAD" convert "$stl" --cadquery "$work/$base.py" 2>&1 | tail -1 || true)
  : > "$work/m2c.txt"
  if [ -f "$work/$base.py" ]; then
    "$MESH2CAD" params "$work/$base.py" | tail -n +2 | awk '{print $1, $2}' | sort > "$work/m2c.txt"
  fi
  mw_summary=$("$root/target/debug/examples/params" "$stl" 2>"$work/mw-report.txt" | sort > "$work/mw.txt"; head -1 "$work/mw-report.txt")
  echo "| parameter | mesh2cad | meshwright | |"
  echo "|---|---|---|---|"
  join -a1 -a2 -e "—" -o 0,1.2,2.2 "$work/m2c.txt" "$work/mw.txt" | while read -r name a b; do
    mark=""
    if [ "$a" = "—" ] || [ "$b" = "—" ]; then
      mark=""
    elif awk -v a="$a" -v b="$b" 'BEGIN { d = a - b; exit !(d > 0.001 || d < -0.001) }' 2>/dev/null; then
      mark="≠"
    fi
    echo "| \`$name\` | $a | $b | $mark |"
  done
  echo
  echo "- mesh2cad: $m2c_verdict"
  echo "- meshwright: $mw_summary"
  echo
done
