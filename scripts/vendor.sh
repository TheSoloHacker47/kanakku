#!/bin/sh
# Copies the map libraries from node_modules into the static assets directory.
set -eu
cd "$(dirname "$0")/.."
mkdir -p assets/vendor
cp node_modules/leaflet/dist/leaflet.js node_modules/leaflet/dist/leaflet.css assets/vendor/
cp node_modules/protomaps-leaflet/dist/protomaps-leaflet.js assets/vendor/
