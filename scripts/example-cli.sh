#!/usr/bin/env bash
# Headless end-to-end example (CI job `example-cli`): install the built plugin zip into a
# scratch plugins dir, import examples/corner-plate.stl through solidcraft-cli's file.import
# with the importer entry `revmesh.stl`, export the rebuilt design as STL, and check both that
# the import rebuilt the part parametrically (its loss report is no mesh fallback) and that the
# rebuilt mesh's volume matches the source mesh within 1 % (the import's own gate).
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

python3 - "$root/examples/corner-plate.stl" "$work/rebuilt-corner-plate.stl" "$work/results.json" <<'PY'
import json, struct, sys

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

def volume(path):
    data = open(path, "rb").read()
    (n,) = struct.unpack("<I", data[80:84])
    if len(data) != 84 + 50 * n:
        raise SystemExit(f"{path}: not a binary STL of {n} triangles")
    v = 0.0
    for i in range(n):
        o = 84 + 50 * i
        a, b, c = (struct.unpack("<3f", data[o + 12 + 12 * j : o + 24 + 12 * j]) for j in range(3))
        v += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.0
    return abs(v)
source_path, rebuilt_path, results_path = sys.argv[1:4]
report = find_report(json.load(open(results_path)))
if report is None:
    raise SystemExit(f"{results_path}: no loss report in the run's results")
summary = report.get("summary", "")
codes = {item.get("code") for item in report.get("items", [])}
print(f"import report: {summary}")
if summary.startswith("A mesh of") or codes & {"revmesh.unrecognized", "revmesh.selfcheck"}:
    raise SystemExit(f"the import fell back to a mesh body, not a parametric rebuild: {summary}")
source, rebuilt = volume(source_path), volume(rebuilt_path)
error = abs(source - rebuilt) / source
print(f"source {source:.6f} mm3, rebuilt {rebuilt:.6f} mm3, volume error {error:.6%}")
if error > 0.01:
    raise SystemExit("volume error above the 1% gate")
PY
echo "example-cli: ok"
