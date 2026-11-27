#!/usr/bin/env bash
# Turns cargo-llvm-cov's JSON summary into the Markdown body of the coverage comment that the
# `coverage-comment` workflow posts on every pull request (CI job `coverage` writes it).
#
#   cargo llvm-cov report --workspace --json --summary-only --output-path coverage.json
#   scripts/coverage-markdown.sh coverage.json <commit-sha> > coverage.md
#
# The first line is a hidden marker the workflow finds its own comment by, so each push updates
# that one comment instead of adding another. Files are listed least covered first; the gate is
# the one the CI job enforces (`--fail-under-lines 80`).
set -euo pipefail
json=${1:?usage: coverage-markdown.sh <coverage.json> [commit-sha]}
sha=${2:-}
gate=${COVERAGE_GATE:-80}
root=$(cd "$(dirname "$0")/.." && pwd)

jq -r --arg root "$root/" --arg sha "$sha" --argjson gate "$gate" '
  def pct: (. * 10 | round) / 10 | tostring | if test("\\.") then . else . + ".0" end;
  def row: "| `\(.name)` | \(.lines.percent | pct) % | \(.lines.covered)/\(.lines.count) | \(.functions.percent | pct) % | \(.regions.percent | pct) % |";
  .data[0] as $d
  | ($d.totals.lines.percent) as $total
  | [ $d.files[] | {name: (.filename | ltrimstr($root)), lines: .summary.lines, functions: .summary.functions, regions: .summary.regions} ]
  | sort_by(.lines.percent, .name) as $files
  | [
      "<!-- revmesh-coverage -->",
      "## Coverage",
      "",
      "\(if $total >= $gate then "✅" else "❌" end) **\($total | pct) %** line coverage for the whole workspace (gate: \($gate) %)\(if $sha != "" then " at \($sha[0:7])" else "" end).",
      "",
      "| | Lines | Covered | Functions | Regions |",
      "|---|---|---|---|---|",
      "| **Total** | **\($total | pct) %** | \($d.totals.lines.covered)/\($d.totals.lines.count) | \($d.totals.functions.percent | pct) % | \($d.totals.regions.percent | pct) % |",
      "",
      "<details><summary>Per file, least covered first (\($files | length) files)</summary>",
      "",
      "| File | Lines | Covered | Functions | Regions |",
      "|---|---|---|---|---|",
      ($files[] | row),
      "",
      "</details>"
    ]
  | .[]
' "$json"
