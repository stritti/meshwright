#!/usr/bin/env python3
"""Renders STL files as PNG previews for the docs' example gallery.

Each STL becomes `<out>/<name>.png`: an isometric view, flat-shaded per facet, with every
triangle edge drawn thin (so the mesh's facets show, as an STL really is) and the sharp edges
(where neighbouring facets turn by more than 30 degrees) drawn dark. Deterministic: the same STL
gives the same PNG.

    pip install numpy pillow
    python3 scripts/render-stl.py --out docs/public/examples examples/*.stl

Needs only numpy and Pillow; no GPU, no display.
"""

from __future__ import annotations

import argparse
import struct
from pathlib import Path

import numpy as np
from PIL import Image

WIDTH, HEIGHT, SUPERSAMPLE = 640, 420, 3
BACKGROUND = (255, 255, 255, 0)
BASE = np.array([86, 132, 196], dtype=float)  # facet colour, lit
FACET_EDGE = np.array([40, 60, 90], dtype=float)
SHARP_EDGE = np.array([20, 28, 40], dtype=float)


def read_stl(path: Path) -> np.ndarray:
    """The triangles of a binary or ASCII STL file, shape (n, 3, 3)."""
    data = path.read_bytes()
    if len(data) >= 84:
        (count,) = struct.unpack_from("<I", data, 80)
        if 84 + 50 * count == len(data):
            rec = np.frombuffer(data, dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]), count=count, offset=84)
            return rec["v"].astype(float)
    verts = [list(map(float, line.split()[1:4])) for line in data.decode("ascii", "replace").splitlines() if line.strip().startswith("vertex")]
    return np.array(verts, dtype=float).reshape(-1, 3, 3)


def view_matrix() -> np.ndarray:
    """Rows: screen right, screen up, towards the viewer, for a view from (+1, -1, +1) rotated
    so that Z stays up, like CAD programs' isometric view from the front right."""
    eye = np.array([1.0, -1.4, 1.0])
    eye /= np.linalg.norm(eye)
    right = np.cross([0.0, 0.0, 1.0], eye)
    right /= np.linalg.norm(right)
    up = np.cross(eye, right)
    return np.stack([right, up, eye])


def sharp_edges(tris: np.ndarray, normals: np.ndarray) -> set[tuple]:
    """Edges whose two facets turn by more than 30 degrees, and open edges."""
    key = lambda p: tuple(np.round(p, 4))  # noqa: E731
    faces: dict[tuple, list[int]] = {}
    for i, t in enumerate(tris):
        for a, b in ((0, 1), (1, 2), (2, 0)):
            e = tuple(sorted((key(t[a]), key(t[b]))))
            faces.setdefault(e, []).append(i)
    cos = np.cos(np.radians(30))
    return {e for e, fs in faces.items() if len(fs) != 2 or float(np.dot(normals[fs[0]], normals[fs[1]])) < cos}


def render(tris: np.ndarray) -> Image.Image:
    w, h = WIDTH * SUPERSAMPLE, HEIGHT * SUPERSAMPLE
    rot = view_matrix()
    normals = np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0])
    lengths = np.linalg.norm(normals, axis=1)
    keep = lengths > 1e-12
    tris, normals = tris[keep], normals[keep] / lengths[keep, None]
    sharp = sharp_edges(tris, normals)

    cam = tris @ rot.T  # x right, y up, z towards the viewer
    lo, hi = cam.reshape(-1, 3).min(axis=0), cam.reshape(-1, 3).max(axis=0)
    scale = 0.88 * min(w / (hi[0] - lo[0]), h / (hi[1] - lo[1]))
    mid = (lo + hi) / 2
    sx = (cam[..., 0] - mid[0]) * scale + w / 2
    sy = h / 2 - (cam[..., 1] - mid[1]) * scale
    sz = cam[..., 2]

    light = np.array([0.35, 0.45, 0.82])
    light /= np.linalg.norm(light)
    view_normals = normals @ rot.T
    shade = 0.35 + 0.65 * np.clip(view_normals @ light, 0, 1)

    zbuf = np.full((h, w), -np.inf)
    rgb = np.zeros((h, w, 3))
    alpha = np.zeros((h, w))
    for i in range(len(tris)):
        if view_normals[i, 2] <= 0:  # back face
            continue
        x, y, z = sx[i], sy[i], sz[i]
        x0, x1 = max(int(np.floor(x.min())), 0), min(int(np.ceil(x.max())), w - 1)
        y0, y1 = max(int(np.floor(y.min())), 0), min(int(np.ceil(y.max())), h - 1)
        if x1 < x0 or y1 < y0:
            continue
        px, py = np.meshgrid(np.arange(x0, x1 + 1) + 0.5, np.arange(y0, y1 + 1) + 0.5)
        d = (y[1] - y[2]) * (x[0] - x[2]) + (x[2] - x[1]) * (y[0] - y[2])
        if abs(d) < 1e-12:
            continue
        b0 = ((y[1] - y[2]) * (px - x[2]) + (x[2] - x[1]) * (py - y[2])) / d
        b1 = ((y[2] - y[0]) * (px - x[2]) + (x[0] - x[2]) * (py - y[2])) / d
        b2 = 1 - b0 - b1
        inside = (b0 >= -1e-9) & (b1 >= -1e-9) & (b2 >= -1e-9)
        depth = b0 * z[0] + b1 * z[1] + b2 * z[2]
        region = zbuf[y0 : y1 + 1, x0 : x1 + 1]
        nearer = inside & (depth > region)
        region[nearer] = depth[nearer]
        rgb[y0 : y1 + 1, x0 : x1 + 1][nearer] = BASE * shade[i] + (255 - BASE) * 0.18 * shade[i]
        alpha[y0 : y1 + 1, x0 : x1 + 1][nearer] = 255

    # Edges: drawn where they are not hidden behind nearer facets.
    eps = (hi[2] - lo[2]) * 1e-3 + 1e-6
    key = lambda p: tuple(np.round(p, 4))  # noqa: E731
    drawn: set[tuple] = set()
    for i in range(len(tris)):
        if view_normals[i, 2] <= 0:
            continue
        for a, b in ((0, 1), (1, 2), (2, 0)):
            e = tuple(sorted((key(tris[i][a]), key(tris[i][b]))))
            if e in drawn:
                continue
            drawn.add(e)
            is_sharp = e in sharp
            n = int(max(abs(sx[i, b] - sx[i, a]), abs(sy[i, b] - sy[i, a]))) + 2
            t = np.linspace(0, 1, n)
            lx = sx[i, a] + (sx[i, b] - sx[i, a]) * t
            ly = sy[i, a] + (sy[i, b] - sy[i, a]) * t
            lz = sz[i, a] + (sz[i, b] - sz[i, a]) * t
            radius = SUPERSAMPLE if is_sharp else SUPERSAMPLE // 2
            colour, weight = (SHARP_EDGE, 1.0) if is_sharp else (FACET_EDGE, 0.35)
            for ox in range(-radius + 1, radius) if radius > 1 else (0,):
                for oy in range(-radius + 1, radius) if radius > 1 else (0,):
                    ix = np.clip((lx + ox).astype(int), 0, w - 1)
                    iy = np.clip((ly + oy).astype(int), 0, h - 1)
                    visible = lz >= zbuf[iy, ix] - eps
                    ix, iy = ix[visible], iy[visible]
                    rgb[iy, ix] = rgb[iy, ix] * (1 - weight) + colour * weight
                    alpha[iy, ix] = 255

    image = np.dstack([rgb, alpha]).clip(0, 255).astype(np.uint8)
    small = Image.fromarray(image, "RGBA").resize((WIDTH, HEIGHT), Image.LANCZOS)
    # Trim the empty margin round the part, keeping a little air.
    left, top, right, bottom = small.getbbox() or (0, 0, WIDTH, HEIGHT)
    pad = 8
    return small.crop((max(left - pad, 0), max(top - pad, 0), min(right + pad, WIDTH), min(bottom + pad, HEIGHT)))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, required=True, help="directory for the PNG files")
    parser.add_argument("stl", type=Path, nargs="+")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    for path in args.stl:
        out = args.out / f"{path.stem}.png"
        render(read_stl(path)).save(out, optimize=True)
        print(f"{path} -> {out}")


if __name__ == "__main__":
    main()
