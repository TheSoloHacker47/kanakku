#!/bin/sh
# Builds the Ernakulam basemap from the Protomaps daily planet build (OpenStreetMap data).
# Needs the pmtiles CLI: brew install pmtiles
set -eu
cd "$(dirname "$0")/.."
mkdir -p tiles
BUILD="${1:-$(date -u -v-1d +%Y%m%d)}"
pmtiles extract "https://build.protomaps.com/$BUILD.pmtiles" tiles/ernakulam.pmtiles \
  --bbox=76.10,9.70,77.00,10.40 --maxzoom=13 --download-threads=8
pmtiles show tiles/ernakulam.pmtiles
echo
echo "Upload for local dev:  npx wrangler r2 object put kanakku/tiles/ernakulam.pmtiles --file tiles/ernakulam.pmtiles --local"
echo "Upload for production: npx wrangler r2 object put kanakku/tiles/ernakulam.pmtiles --file tiles/ernakulam.pmtiles --remote"
