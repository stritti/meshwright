#!/usr/bin/env bash
# Headless end-to-end example (CI job `example-cli`): install the built plugin zip into a
# scratch plugins dir, import examples/corner-plate.stl through solidcraft-cli's file.import
# with the importer entry `revmesh.stl`, export the rebuilt design as STL, and check both that
# the import rebuilt the part parametrically (its loss report is no mesh fallback) and that the
# rebuilt mesh's volume matches the source mesh within 1 % (the import's own gate, checked
# with scripts/measure-stl.py --gate).
#
#   cargo xtask dist          # target/dist/revmesh-<version>.zip
#   scripts/example-cli.sh    # SOLIDCRAFT_CLI=... or solidcraft-cli on PATH
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cli=${SOLIDCRAFT_CLI:-solidcraft-cli}
version=$(python3 -c 'import json; print(json.load(open("'"$root"'/plugin/revmesh/plugin.json"))["version"])')
zip="$root/target/dist/revmesh-$version.zip"
if [ ! -f "$zip" ]; then
  echo "no $zip; run cargo xtask dist first" >&2
  exit 2
fi
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
plugins=$work/plugins

approval=$("$cli" --plugins-dir "$plugins" exec plugin.check "{\"path\": \"$zip\"}" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["approval"])')
echo "package $zip, approval $approval"

sed -e "s|../../target/dist/revmesh-<version>.zip|$zip|" \
    -e "s|<approval hash from check-package.json / plugin.check>|$approval|" \
    -e "s|\.\./corner-plate\.stl|$root/examples/corner-plate.stl|" \
    -e "s|\.\./\.\./target/cli-example/rebuilt-corner-plate\.stl|$work/rebuilt-corner-plate.stl|" \
    "$root/examples/cli/import-corner-plate.json" > "$work/import.json"

"$cli" --plugins-dir "$plugins" run "$work/import.json" --results --json-out "$work/results.json"

python3 - "$work/results.json" <<'PY'
import json, sys

def find_report(node):
    """The importer's loss report, wherever the run's results nest it."""
    if isinstance(node, dict):
        if isinstance(node.get("loss_report"), dict):
            return node["loss_report"]
        if "summary" in node and "items" in node:
            return node
        for value in node.values():
            found = find_report(value)
            if found is not None:
                return found
    elif isinstance(node, list):
        for value in node:
            found = find_report(value)
            if found is not None:
                return found
    return None

results_path = sys.argv[1]
report = find_report(json.load(open(results_path)))
if report is None:
    raise SystemExit(f"{results_path}: no loss report in the run's results")
summary = report.get("summary", "")
codes = {item.get("code") for item in report.get("items", [])}
print(f"import report: {summary}")
if summary.startswith("A mesh of") or codes & {"revmesh.unrecognized", "revmesh.selfcheck"}:
    raise SystemExit(f"the import fell back to a mesh body, not a parametric rebuild: {summary}")
PY

python3 "$root/scripts/measure-stl.py" --gate 0.01 "$root/examples/corner-plate.stl" "$work/rebuilt-corner-plate.stl"
echo "example-cli: ok"
