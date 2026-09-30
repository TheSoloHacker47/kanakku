# Kanakku (കണക്ക്)

Public project accountability for Kerala. This first build covers KIIFB projects in Ernakulam district: what was sanctioned, who got the contract, what has been paid, how far the work has come, and a copy of the source page behind every figure.

The product spec is in [kanakku-spec.md](kanakku-spec.md). The design of this build is in [docs/design.md](docs/design.md).

## How it is built

Everything runs on Cloudflare, written in Rust.

| Part | What it does |
|---|---|
| `crates/core` | Pure Rust: the KIIFB parser, flag rules, change diffing, Indian number formatting and the Malayalam/English strings. Tested natively. |
| `crates/worker` | The Worker (`workers-rs`): server-rendered pages, open-data endpoints and the nightly ingest. |
| `migrations/` | D1 schema. |
| `assets/` | Static files: fonts, icons, the share image, and the map script with its two libraries. Pages other than the map load no JavaScript. |

A page is one HTML request with its stylesheet inlined and one D1 round trip. Fonts are self-hosted and cached for a year.

## Run it locally

You need Rust with the `wasm32-unknown-unknown` target, `worker-build` (`cargo install worker-build`) and Node.

```bash
npm install
npx wrangler d1 migrations apply kanakku --local
npm run dev
```

`.dev.vars` needs an `INGEST_TOKEN` of your choosing, and `EDGE_CACHE=off` so edits show up immediately. Then load the data:

```bash
curl -X POST -H "Authorization: Bearer $INGEST_TOKEN" http://localhost:8787/admin/ingest
```

The map needs a basemap file in R2. `scripts/basemap.sh` builds it and explains how to upload it.

## Generated assets

These are committed, so you only need the scripts when something changes.

| Script | Writes | Needs |
|---|---|---|
| `scripts/icons.py` | `crates/worker/src/icons.rs` from Lucide | `npm install` |
| `scripts/district.py` | `crates/worker/src/district.rs`, the district outline from OpenStreetMap | network |
| `scripts/brand.py` | share image and app icons in `assets/` | Google Chrome, running dev server |
| `scripts/vendor.sh` | map libraries in `assets/vendor/` | `npm install` |
| `scripts/basemap.sh` | `tiles/ernakulam.pmtiles` | `brew install pmtiles` |

`scripts/shot.sh URL out.png WIDTH HEIGHT` takes a screenshot with headless Chrome. It cannot go narrower than about 500 px.

## Tests

```bash
cargo test -p kanakku-core
```

To see what an ingest would store from a saved copy of the dashboard:

```bash
cargo run -p kanakku-core --release --example inspect -- page.html Ernakulam 2026-09-30
```

## Ingest

A cron trigger runs at 02:30 IST. It fetches `https://gis.kiifb.org/` once, and only that page. If the data has changed it stores the page in R2, updates the projects that changed and records each changed field. Flags are recomputed on every run, because "overdue" depends on today's date.

If KIIFB blocks Cloudflare's addresses, fetch the page from a machine in India and push it instead:

```bash
curl -A "KanakkuBot/0.1" https://gis.kiifb.org/ -o page.html
curl -X POST -H "Authorization: Bearer $INGEST_TOKEN" --data-binary @page.html https://<your-domain>/admin/ingest
```
