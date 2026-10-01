#!/bin/sh
# Builds the Kerala basemap from the Protomaps daily planet build (OpenStreetMap data).
# Needs the pmtiles CLI: brew install pmtiles
set -eu
cd "$(dirname "$0")/.."
mkdir -p tiles
BUILD="${1:-$(date -u -v-1d +%Y%m%d)}"
pmtiles extract "https://build.protomaps.com/$BUILD.pmtiles" tiles/kerala.pmtiles \
  --bbox=74.80,8.10,77.50,12.90 --maxzoom=12 --download-threads=3
pmtiles show tiles/kerala.pmtiles
echo
echo "Upload for local dev:  npx wrangler r2 object put kanakku/tiles/kerala.pmtiles --file tiles/kerala.pmtiles --local"
echo "Then set the date in TILES in assets/map.js to $BUILD, so browsers drop their cached copy."
echo "Upload for production: npx wrangler r2 object put kanakku/tiles/kerala.pmtiles --file tiles/kerala.pmtiles --remote"
