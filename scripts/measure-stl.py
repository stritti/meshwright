#!/usr/bin/env python3
"""Measures STL files (binary or ASCII): triangle count, volume, surface area, bounding box.

    python3 scripts/measure-stl.py part.stl [more.stl ...]
    python3 scripts/measure-stl.py --json part.stl
    python3 scripts/measure-stl.py --gate 0.01 source.stl rebuilt.stl

Stdlib only. Volume is the signed-triangle sum (divergence theorem), so it assumes a closed,
consistently oriented mesh, as CAD exporters write. `--gate FRAC` compares two files' volumes
relatively and exits 1 above FRAC — the same 1% gate the import itself applies to a rebuilt
part (`cargo xtask check`), so `scripts/example-cli.sh` uses this to verify a rebuilt STL.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import sys
from pathlib import Path


def read_stl(path: Path) -> list[tuple[tuple[float, float, float], ...]]:
    data = path.read_bytes()
    if len(data) >= 84:
        (count,) = struct.unpack("<I", data[80:84])
        if len(data) == 84 + 50 * count:
            tris = []
            for i in range(count):
                tri = tuple(
                    struct.unpack("<3f", data[84 + 50 * i + 12 + 12 * j : 84 + 50 * i + 24 + 12 * j]) for j in range(3)
                )
                for vertex in tri:
                    if not all(math.isfinite(c) for c in vertex):
                        raise ValueError(f"{path}: triangle {i + 1} has a corner that is not a finite number")
                tris.append(tri)
            return tris
    # Not a size-matching binary file: try ASCII (some binary files start with "solid" too;
    # the size check above is the decider, as in revmesh's own stl.rs).
    text = data.decode("utf-8", errors="replace")
    if not text.lstrip().startswith("solid"):
        if len(data) >= 84:
            raise ValueError(f"{path}: the binary STL header announces {count} triangles, but the file has only {len(data)} bytes")
        raise ValueError(f"{path}: too short to be an STL file")
    verts: list[tuple[float, float, float]] = []
    tris: list[tuple[tuple[float, float, float], ...]] = []
    for lineno, line in enumerate(text.splitlines(), 1):
        parts = line.split()
        if parts and parts[0] == "vertex":
            if len(parts) != 4:
                raise ValueError(f"{path}: line {lineno}: a vertex has three coordinates")
            try:
                vertex = tuple(float(p) for p in parts[1:4])
            except ValueError:
                raise ValueError(f"{path}: line {lineno}: bad coordinate") from None
            if not all(math.isfinite(c) for c in vertex):
                raise ValueError(f"{path}: line {lineno}: not a finite number")
            verts.append(vertex)
            if len(verts) == 3:
                tris.append(tuple(verts))
                verts = []
    if not tris:
        raise ValueError(f"{path}: holds no triangles")
    if verts:
        raise ValueError(f"{path}: ends inside a facet")
    return tris


def measure(tris: list[tuple[tuple[float, float, float], ...]]) -> dict:
    volume = 0.0
    area = 0.0
    lo = [math.inf] * 3
    hi = [-math.inf] * 3
    for a, b, c in tris:
        volume += (
            a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])
        ) / 6.0
        u = tuple(b[i] - a[i] for i in range(3))
        v = tuple(c[i] - a[i] for i in range(3))
        n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
        area += math.sqrt(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]) / 2.0
        for i in range(3):
            lo[i] = min(lo[i], a[i], b[i], c[i])
            hi[i] = max(hi[i], a[i], b[i], c[i])
    return {"triangles": len(tris), "volume_mm3": abs(volume), "surface_area_mm2": area, "bbox_mm": {"min": lo, "max": hi}}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("files", nargs="+", type=Path, help="STL files to measure")
    ap.add_argument("--json", action="store_true", help="print JSON instead of text")
    ap.add_argument("--gate", type=float, metavar="FRAC", help="with exactly two files: exit 1 when their relative volume error exceeds FRAC")
    args = ap.parse_args()

    # Keyed by full path, not basename: --gate with a/part.stl b/part.stl must not collapse.
    results: dict[str, dict] = {}
    for f in args.files:
        try:
            results[str(f)] = measure(read_stl(f))
        except (ValueError, OSError) as e:
            print(f"error: {e}", file=sys.stderr)
            return 2

    if args.gate is not None:
        if len(args.files) != 2:
            print("error: --gate needs exactly two files", file=sys.stderr)
            return 2
        src, rebuilt = (results[str(f)] for f in args.files)
        if not (math.isfinite(src["volume_mm3"]) and math.isfinite(rebuilt["volume_mm3"])) or src["volume_mm3"] <= 0:
            print("error: source or rebuilt volume is not a finite positive number", file=sys.stderr)
            return 2
        error = abs(src["volume_mm3"] - rebuilt["volume_mm3"]) / src["volume_mm3"]
        print(f"source {src['volume_mm3']:.6f} mm3, rebuilt {rebuilt['volume_mm3']:.6f} mm3, volume error {error:.6%}")
        if args.json:
            print(json.dumps({"source": src, "rebuilt": rebuilt, "volume_error": error}))
        return 1 if error > args.gate else 0

    if args.json:
        print(json.dumps(results, indent=1))
    else:
        for name, r in results.items():
            print(
                f"{name}: {r['triangles']} triangles, volume {r['volume_mm3']:.6f} mm3, "
                f"surface {r['surface_area_mm2']:.6f} mm2, bbox {r['bbox_mm']['min']} .. {r['bbox_mm']['max']} mm"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
