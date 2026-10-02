# Kanakku (കണക്ക്)

Public project accountability for Kerala: what was sanctioned, who got the contract, what has been paid, how far the work has come. It covers KIIFB projects in all 14 districts from two KIIFB sources, and the Public Works Department's list of finished works still under contractor liability. Every number links to a stored copy of its source.

Live at **https://keralakanakku.com** (Malayalam) and https://keralakanakku.com/en/ (English). `www.` and the old `kanakku.thesolohacker47.workers.dev` address redirect there; `POST /admin/ingest` works on either host.

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
| `scripts/basemap.sh` | `tiles/kerala.pmtiles` | `brew install pmtiles` |
| `scripts/districts.py` | `crates/worker/src/district.rs` (district outlines) | Python 3 |
| `scripts/constituencies.py` | `crates/core/src/constituencies.rs` | Python 3, a saved dashboard page |
| `scripts/ogfonts.py` | `crates/worker/fonts/*.ttf` (share-image fonts) | `pip install fonttools brotli` |

`scripts/shot.sh URL out.png WIDTH HEIGHT` takes a screenshot with headless Chrome. It cannot go narrower than about 500 px.

## Tests

```bash
cargo test -p kanakku-core
```

To see what an ingest would store from a saved copy of the dashboard:

```bash
cargo run -p kanakku-core --release --example inspect -- page.html '*' 2026-09-30
```

## Ingest

A cron trigger runs at 02:30 IST. It fetches `https://gis.kiifb.org/` once, and only that page. If the data has changed it stores the page in R2, updates the projects that changed and records each changed field. Flags are recomputed on every run, because "overdue" depends on today's date.

The same trigger then reads KIIFB's project status page (`https://www.kiifb.org/prjStatus.jsp`) through its public form: one request for the district list, then one per project whose figures changed, plus ten of the longest-unchecked, with a second's pause between requests. To run it by hand, capped at 40 work tables:

```sh
curl -X POST -H "Authorization: Bearer $INGEST_TOKEN" "https://<your-domain>/admin/ingest?source=status&details=40"
```

Once a week it also reads the Kerala PWD defect-liability list for the district's divisions (22 requests). To force a read:

```sh
curl -X POST -H "Authorization: Bearer $INGEST_TOKEN" "https://<your-domain>/admin/ingest?source=liability"
```

If KIIFB blocks Cloudflare's addresses, fetch the page from a machine in India and push it instead:

```bash
curl -A "KanakkuBot/0.1" https://gis.kiifb.org/ -o page.html
curl -X POST -H "Authorization: Bearer $INGEST_TOKEN" --data-binary @page.html https://<your-domain>/admin/ingest
```

## Scope, alerts and health

- `PILOT_DISTRICT` in `wrangler.toml` is `*` for the whole state, or one district's name.
- Every ingest run is logged. `/status` shows the last good read of each source; `/api/v1/status` answers 503 when a source is stale, so any uptime monitor can watch it.
- To be told when a run fails, set a webhook (Slack, Discord or similar): `npx wrangler secret put ALERT_WEBHOOK`.
- Visit counts are per kind of page, language and day. No cookies, addresses or identifiers.
- Searches that find nothing are kept as words and a daily count (`search_misses`), never with who searched. Text that looks like an email address or phone number is not kept. To read them:
  `npx wrangler d1 execute kanakku --remote --command "SELECT q, surface, SUM(n) AS n FROM search_misses GROUP BY q, surface ORDER BY n DESC LIMIT 50"`

## Pages

| Path | What |
|---|---|
| `/`, `/d/{district}` | Front page for the state and for a district |
| `/projects`, `/p/{code}` | Dashboard projects; search works in Malayalam |
| `/funding`, `/f/{ref}` | What KIIFB approved and what has been paid |
| `/liability` | Finished PWD works still under contractor liability |
| `/contractors`, `/c/{key}` | Contractors across the sources |
| `/agencies`, `/a/{key}` | Implementing agencies |
| `/og/p/{code}.png`, `/og/f/{ref}.png` | Share images, drawn on request |
| `/status`, `/methodology`, `/data`, `/map` | Health, method, downloads, map |
| `/about` | Who runs it, corrections, takedown requests, privacy, licences. The contact shown is `CONTACT` in `wrangler.toml` |

## Open data API

Everything is under `/api/v1/`, described in full at `/api/v1/openapi.json` (a static file in `assets/api/v1/`). Full downloads: `projects`, `funding`, `liability` (JSON and `.csv`), `projects.geojson`, `projects/{code}`. Paged, 500 rows a page with a `next` link: `changes?since=yyyy-mm-dd`, `flags?status=open|cleared|all&type=…`, `snapshots?source=1|2|3`; all three take `district=`. Version 1 only grows; a breaking change would be `/api/v2/`.

## Licences

- Code: [AGPL-3.0-or-later](LICENSE).
- Data: what Kanakku adds (the compilation, cleaned names, stages, flags and joins) is [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). The figures are the sources'. Every JSON download carries `license` and `attribution`.
