#!/usr/bin/env python3
"""Renders the share image and app icons with headless Chrome, using the site's own fonts.

Needs Google Chrome and a running dev server (for the project locations):
    scripts/brand.py [http://localhost:8787]
Writes assets/og.png, assets/icon-512.png, assets/icon-192.png and assets/icon-maskable.png.
"""
import json, pathlib, re, subprocess, sys, tempfile, urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
BASE = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:8787"

district = (ROOT / "crates/worker/src/district.rs").read_text()
# The whole state, in the box district.rs draws it in.
state = re.search(r"pub const STATE: Frame = Frame \{ width: ([\d.]+), height: ([\d.]+), min_lng: ([\d.]+), max_lat: ([\d.]+), kx: ([\d.]+), ky: ([\d.]+) \}", district)
WIDTH, HEIGHT, MIN_LNG, MAX_LAT, KX, KY = (float(v) for v in state.groups())
PAD = float(re.search(r"const PAD: f64 = ([\d.]+);", district).group(1))
OUTLINES = "".join(f'<path class="land" d="{d}"/>' for d in re.findall(r'in_state: "([^"]+)"', district))

features = json.load(urllib.request.urlopen(f"{BASE}/api/v1/projects.geojson"))["features"]
dots = []
for f in sorted(features, key=lambda f: f["properties"]["flagged"]):
    lng, lat = f["geometry"]["coordinates"]
    x, y = PAD + (lng - MIN_LNG) * KX, PAD + (MAX_LAT - lat) * KY
    if 0 <= x <= WIDTH and 0 <= y <= HEIGHT:
        flagged = f["properties"]["flagged"]
        dots.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{2.6 if flagged else 1.1}" class="{"f" if flagged else ""}"/>')

MARK = '<path d="M9 8v16M13.7 8v16M18.3 8v16M23 8v16M5.5 20.5l21-9" fill="none" stroke="#0a0a0a" stroke-width="2.3" stroke-linecap="round"/>'
FONTS = f"""
@font-face {{ font-family: D; src: url(file://{ASSETS}/fonts/anek-display-800-malayalam.woff2); unicode-range: U+0D00-0D7F, U+200C-200D; }}
@font-face {{ font-family: D; src: url(file://{ASSETS}/fonts/anek-display-800-latin.woff2); }}
@font-face {{ font-family: T; src: url(file://{ASSETS}/fonts/anek-text-600-malayalam.woff2); unicode-range: U+0D00-0D7F, U+200C-200D; }}
@font-face {{ font-family: T; src: url(file://{ASSETS}/fonts/anek-text-600-latin.woff2); }}
"""

og = f"""<!doctype html><meta charset="utf-8"><style>{FONTS}
html, body {{ margin: 0; }}
body {{ width: 1200px; height: 630px; background: #44d991; color: #0a0a0a; display: grid; grid-template-columns: 1.1fr 0.9fr; align-items: center; gap: 20px; padding: 0 64px; box-sizing: border-box; }}
.brand {{ display: flex; align-items: center; gap: 16px; font: 800 56px/1 D; }}
.brand svg {{ width: 64px; height: 64px; }}
h1 {{ font: 800 96px/1.14 D; margin: 36px 0 20px; }}
p {{ font: 600 30px/1.35 T; margin: 0; }}
.land {{ fill: rgba(255,255,255,.5); stroke: #0a0a0a; stroke-width: .8; stroke-linejoin: round; }}
circle {{ fill: #0a0a0a; }} .f {{ fill: #ff6a51; stroke: #0a0a0a; stroke-width: 1.2; }}
</style>
<div>
  <div class="brand"><svg viewBox="0 0 32 32"><rect width="32" height="32" rx="7" fill="#0a0a0a"/>{MARK.replace('#0a0a0a', '#44d991')}</svg>കണക്ക്</div>
  <h1>പൊതുപണം എവിടെ പോകുന്നു?</h1>
  <p>Where the public money goes. Public projects across Kerala, with their sources.</p>
</div>
<svg viewBox="0 0 {WIDTH} {HEIGHT}">{OUTLINES}{''.join(dots)}</svg>
"""

def icon(padding):
    inner = 512 - 2 * padding
    radius = 0 if padding else 112
    return f"""<!doctype html><meta charset="utf-8"><style>html, body {{ margin: 0; background: {'#44d991' if padding else 'transparent'}; }}</style>
<svg width="512" height="512" viewBox="0 0 512 512"><rect width="512" height="512" rx="{radius}" fill="#44d991"/>
<svg x="{padding}" y="{padding}" width="{inner}" height="{inner}" viewBox="0 0 32 32">{MARK}</svg></svg>"""

def shot(html, out, width, height):
    with tempfile.NamedTemporaryFile("w", suffix=".html", delete=False) as f:
        f.write(html)
    subprocess.run(
        [CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--default-background-color=00000000",
         f"--window-size={width},{height}", "--virtual-time-budget=3000", f"--screenshot={out}", f"file://{f.name}"],
        check=True, capture_output=True)

shot(og, ASSETS / "og.png", 1200, 630)
shot(icon(0), ASSETS / "icon-512.png", 512, 512)
shot(icon(64), ASSETS / "icon-maskable.png", 512, 512)
subprocess.run(["sips", "-Z", "192", str(ASSETS / "icon-512.png"), "--out", str(ASSETS / "icon-192.png")], check=True, capture_output=True)
print("wrote", ", ".join(f"{p.name} ({p.stat().st_size // 1024} KB)" for p in sorted(ASSETS.glob("*.png"))))
