# Kanakku build 1: KIIFB MVP slice, all Rust on Cloudflare

## Context

`kanakku-spec.md` describes a 5-phase public project accountability platform for Kerala. It is too large for one build, and its stack (React + Hono) is heavier than you want. You chose: **all-Rust backend**, **KIIFB MVP slice** first, **Workers Paid, deployed**.

What the research found:

- **Rust on Workers** (`worker` crate 0.8.7, Sept 2026) compiles to WASM inside V8. Per request it is on par with JS, not faster: slightly slower on trivial requests, slightly faster on heavy parsing. It has bindings for D1, R2, KV, Queues, Cron and Durable Objects, but none for Workflows, Browser Rendering or Vectorize. None of those three are needed in this build.
- **Where the speed comes from:** server-rendered HTML, edge caching, static assets served before the Worker runs, and almost no client JS. No React, no Hono, no Tailwind, no web fonts.
- **KIIFB needs no headless browser.** `https://gis.kiifb.org/` embeds everything as JSON in one 12.6 MB page: `MARKERS` (3,267 pins, 3,074 project codes; Ernakulam has 283 pins for 263 projects, plus 43 projects that appear only as transport works) and `TRANSPORT_GEOJSON` (699 road/bridge works, 57 in Ernakulam, with contractor, start/finish dates, paid amount, physical and financial progress).
- Data quirks to handle: a project code can be listed under several constituencies (same pin repeated); marker amounts are in rupees despite the "(Cr)" label, transport amounts are in crore; only 141 of 699 transport works share a code with a marker.
- Today's data would raise 4 "Overdue" flags in Ernakulam and 0 "Payment vs progress" flags.
- Sulekha loads from your machine. eTenders, CPPP and kerala.data.gov.in timed out. They belong to a later build.
- **Do not touch:** `gis.kiifb.org/projects/kiifb_dataupdate.php` answers an unauthenticated GET with a "Project Data Admin, update records directly" page. The ingest reads only the public dashboard page and never calls that endpoint. Worth reporting to KIIFB or CERT-In.

## Scope

In: KIIFB Ernakulam ingest with R2 snapshots, D1 schema, flag engine, project list with search and filters, project page, methodology page, map, Malayalam default with English toggle, open JSON and CSV.

Out (later builds, each with its own plan): Sulekha and tender scrapers, cross-source matching, RTI Centre (adds a small TS Worker for Workflows), citizen photos, accounts, moderation, PWA offline, OG images.

## Architecture

```
kanakku/
  Cargo.toml                  workspace
  crates/core/                pure Rust, no wasm deps, tested natively
    src/kiifb.rs              slice MARKERS / TRANSPORT_GEOJSON out of the page, serde into typed structs, normalise
    src/model.rs              Project, Site, Work, Flag
    src/flags.rs              versioned rules -> Vec<Flag> with evidence
    src/fmt.rs                Indian number format (1,23,45,678), lakh/crore, dates
    src/i18n.rs               ml / en string tables (compile-time)
    tests/fixtures/           trimmed gis.kiifb.org snapshot
  crates/worker/              workers-rs
    src/lib.rs                #[event(fetch)] router, #[event(scheduled)] nightly ingest
    src/db.rs                 D1 queries
    src/ingest.rs             fetch -> sha256 -> R2 snapshot -> parse -> D1 batch upsert -> flags
    src/views/*.rs            maud templates (compiled HTML, no runtime template engine)
    src/api.rs                JSON, CSV, GeoJSON
  assets/                     Workers static assets: app.css, app.js, map.js, vendored maplibre + pmtiles
  migrations/0001_init.sql
  wrangler.toml               D1, R2, assets, cron, observability
  docs/design.md              this design, committed
```

Decisions:

- **Routes.** `/` and `/en/` list; `/p/{code}` project; `/map`; `/methodology`; `/api/v1/projects`, `/api/v1/projects/{code}`, `/api/v1/projects.csv`, `/api/v1/projects.geojson`; `/tiles/ernakulam.pmtiles` (R2 with Range); `POST /admin/ingest` guarded by a secret header for manual runs.
- **Language in the URL** (`/` Malayalam, `/en/` English) so every page is cacheable by URL. KIIFB titles exist only in English; UI strings, constituency and MLA names are Malayalam.
- **Caching.** HTML and API responses go through the Cache API with `s-maxage=300, stale-while-revalidate`, plus ETag. Data changes nightly, so 5 minutes of staleness is fine. Whether the Cache API is active on `workers.dev` gets checked at deploy; if not, it starts working once a custom domain is attached.
- **Fonts.** System stack only (Android ships Noto Sans Malayalam, iOS ships Malayalam Sangam MN). Zero font bytes.
- **Client JS budget.** Under 5 KB on list and project pages. Filters are a plain GET form that works without JS.
- **Search.** D1 FTS5 with the trigram tokenizer over title, code, constituency, executing agency, contractor. Malayalam transliteration search is deferred.
- **Map.** MapLibre GL + PMTiles, loaded only on `/map` (about 220 KB compressed, which is why it stays off every other page). Ernakulam basemap extract on R2. Map labels in English, since MapLibre GL JS does not shape Malayalam script correctly. Pins coloured by flag status.
- **Ingest.** Cron at 02:30 IST. Identifying User-Agent. Hash the two extracted JSON segments; skip if unchanged. Only `PILOT_DISTRICT=Ernakulam` rows are stored. D1 writes go in batches.
- **Money** stored as integer rupees.

### D1 schema (v1)

```sql
sources(id, name, base_url, kind, license, last_scraped_at)
snapshots(id, source_id, url, r2_key, sha256, fetched_at)
projects(id, code UNIQUE, title_en, department, sector, executing_agency, funding_source,
         district, estimated_amount, expenditure, official_status, first_seen_at, updated_at, snapshot_id)
project_constituencies(project_id, name, name_ml, mla_name, mla_name_ml)
sites(id, project_id, lat, lng)
works(id, project_id, road_name, spv, contractor_name,
      as_amount, fs_amount, ts_amount, tender_amount, contract_amount, paid_amount,
      scheduled_start, scheduled_end, progress_note, physical_pct, financial_pct,
      status, lat, lng, snapshot_id)
observations(id, project_id, snapshot_id, field, old_value, new_value, observed_at)
flags(id, project_id, work_id NULL, type, rule_version, value_json, evidence_json,
      status, created_at, cleared_at)
audit_log(id, actor, action, entity, entity_id, diff_json, at)
projects_fts  -- FTS5 trigram
```

Tenders, awards, RTI, users and photos tables arrive with their own builds.

### Flags in this build

| Flag | Rule v1 | Data |
|---|---|---|
| Overdue | today > `scheduled_end` and status is not Completed | works |
| Payment vs progress | `financial_pct - physical_pct >= 25` | works |
| Cost escalation | latest estimated amount / first recorded amount >= 1.2 | observations (fires once amounts change across snapshots) |
| Stale | no field change for 90+ days | observations (fires from day 90) |

Low competition, photo contradiction and audit mention need sources that come later. Each flag row stores its rule version and the snapshot it was derived from, and the project page links to that snapshot.

## Build order

1. **Setup.** `git init`, `rustup update stable`, add `wasm32-unknown-unknown`, install `worker-build`, add `wrangler` as a dev dependency. Commit `docs/design.md`.
2. **`crates/core`, test-first.** Fixture from the saved snapshot. Tests for: segment extraction, rupee/crore normalisation, multi-pin projects, date parsing (`dd-mm-yyyy`), each flag rule including boundaries, Indian number formatting.
3. **Migrations and ingest.** Run against local D1 and R2 under `wrangler dev`; confirm 283 pins and 57 works land, and that a second run is a no-op.
4. **List page and project page** with filters, search, pagination, flags, source links, ml/en.
5. **Methodology page and open data** endpoints.
6. **Map** page, PMTiles extract, tile route.
7. **Caching, headers, size pass.** CSP, ETag, compression check, wasm size check.
8. **Deploy.** I stop and ask before creating D1/R2 and deploying to your Cloudflare account.

## Verification

- `cargo test -p kanakku-core` passes.
- `wrangler dev`, trigger ingest, then check in the browser pane at 360 px: list, a flagged project (one of the 4 overdue works), search, filters, language toggle, map.
- Spot-check 5 projects against the live KIIFB dashboard, including a multi-pin project and the Vyttila flyover work.
- Measure transfer sizes: list and project pages under 30 KB total with under 5 KB JS; report the wasm bundle size and cold-start time honestly.
- After deploy: repeat the checks on the live URL, confirm the cron run in Workers logs, and report whether cache hits occur.

## What changed during the build

- **Estimates are sub-project figures.** KIIFB repeats a sub-project's estimate on every contract package under it (one dialysis-centre electrical package carried the whole 148-package programme estimate of about ₹51 crore). The parser counts the packages that share each estimate statewide. A shared estimate is never a project's headline figure and is never summed; the page shows the package's own expenditure, the sub-project estimate labelled as shared, and spending across all its packages.
- **Transport line segments are merged.** One contract drawn as several line segments repeats the same figures; those become one work.
- **Map uses Leaflet + protomaps-leaflet, not MapLibre GL.** About 85 KB compressed instead of about 300 KB, no WebGL needed, and no glyph files to host.
- **List and project pages load no JavaScript at all.** The stylesheet is inlined, so a page is one request and one D1 round trip.
- **Schema additions:** `projects.sub_project_code`, `estimate_shared_by`, `headline_amount`, `missing_since`, `record_json`; `works.work_ref`, `loa_amount`, `paid_contractor`; flags reference a work by `work_ref`.
- **Build notes:** `strip = true` in the release profile breaks `wasm-bindgen`; `watch_dir` must not include the build output directory.
