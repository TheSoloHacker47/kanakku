#!/usr/bin/env python3
"""Builds the small TrueType files the share-image renderer embeds (crates/worker/fonts/).

The site's fonts are WOFF2 subsets; the renderer needs plain TrueType. This takes the Latin
letters from the Latin subsets and the rupee sign from the Malayalam subsets.
Needs fonttools and brotli:  python3 -m venv venv && venv/bin/pip install fonttools brotli
Usage: venv/bin/python scripts/ogfonts.py
"""
import pathlib
from fontTools import subset
from fontTools.ttLib import TTFont

root = pathlib.Path(__file__).resolve().parent.parent
out = root / "crates/worker/fonts"
out.mkdir(exist_ok=True)
LATIN = list(range(0x20, 0x7F)) + [0xB7]

def build(source, name, codepoints):
    font = TTFont(root / "assets/fonts" / source)
    options = subset.Options(layout_features=[], notdef_outline=True, name_IDs=[1, 2], hinting=False)
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=codepoints)
    subsetter.subset(font)
    font.flavor = None
    target = out / name
    font.save(target)
    print(f"{name}: {target.stat().st_size} bytes")

build("anek-display-800-latin.woff2", "display.ttf", LATIN)
build("anek-display-800-malayalam.woff2", "display-rupee.ttf", [0x20B9])
build("anek-text-600-latin.woff2", "text.ttf", LATIN)
build("anek-text-600-malayalam.woff2", "text-rupee.ttf", [0x20B9])
