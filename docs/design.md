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

## Redesign (build 2)

The first build was a black-and-white ledger. It was fast and honest but read as a prototype. This pass gives the site an identity, a front page, and better ways in, without giving up the one-request pages.

### Direction

Researched with Refero. The primary reference is a civic co-operative site (cqcm.coop): white canvas, black ink, confident headlines, and flat colour blocks that carry statistics. Two details are borrowed: heavy condensed display type for headlines and figures (wise.com), and a full-bleed colour block as the hero (n26.com).

| Decision | Source | Why |
|---|---|---|
| White canvas, black ink, black pill buttons, 8 px radius, no shadows | cqcm.coop | Civic and direct; nothing decorative |
| Stat tiles as stacked colour blocks | cqcm.coop | Its signature; the numbers are the content here |
| Condensed extra-bold headlines and figures | wise.com | Long Malayalam headlines fit a phone; figures read at a glance |
| Green hero block | n26.com, cqcm.coop | One strong brand moment per page |
| Anek Malayalam for both scripts | Constraint: Malayalam-first | One family drawn for Malayalam and Latin together, with a condensed extra-bold cut |
| Colour roles | cqcm.coop palette | Green = money and brand. Blue = sources and information. Persimmon = flags, and only flags |
| Tally-of-five mark | Product name (കണക്ക്, "accounts") | The oldest way of keeping count |
| District dot map as the hero image | Own data | Real project locations on the real boundary; no stock imagery |

Rejected on purpose: indigo or violet accents, cream-and-serif "editorial" styling, cards as default containers, dark mode by default, emoji.

### What was added

- **Front page:** search, four stat tiles, flagged projects, departments with bars, constituency tiles, a stage breakdown, most spent, and three lines on how to read the site.
- **Project list at `/projects`:** sorting, a stage filter, removable filter chips, and a heading that summarises the selection (a constituency shows its MLA as KIIFB lists them).
- **Project page:** the headline figure on green, a stage track, a locator map, flag blocks, a schedule line with today marked, other packages under the same sub-project, and a WhatsApp share link.
- **Readable titles.** Filing prefixes are dropped and all-capital titles are calmed; the name as published stays on the page.
- **Plain stages.** KIIFB's workflow labels are grouped into five stages. The grouping is ours and the methodology page says so.
- **One spelling per constituency,** with Malayalam names. KIIFB wrote Vypin three ways.
- **Malayalam department names.**
- **Share image, app icons and a web manifest.**
- **Map deep links:** `/map#CODE` opens on that project.

### Assets and their sources

| Asset | Source | Licence |
|---|---|---|
| Anek Malayalam (three cuts, 120 KB) | Ek Type, via Google Fonts | OFL, text in `assets/fonts/OFL.txt` |
| Icons (36) | Lucide | ISC |
| District outline | OpenStreetMap relation 3740342 | ODbL, credited in the footer |
| Mark, share image, app icons | Made here (`scripts/brand.py`) | Project's own |

### Cost

Pages other than the map still load no JavaScript. A first visit now also fetches the fonts (about 120 KB, cached for a year). HTML grew from about 5 KB to 9 to 14 KB compressed because the stylesheet is larger and the front page carries the dot map.

## Build 3: what KIIFB approved and released (PER-849)

A second source: `https://www.kiifb.org/prjStatus.jsp`. Found during the tender spike (`docs/tender-spike.md`).

**What it adds.** For each project KIIFB lists under the district: the amount approved, the amount released, and a table of works with approved and paid amounts. 140 projects and 529 works for Ernakulam on 30 Sep 2026.

**How it is read** (`crates/worker/src/funding.rs`)

- One form post lists the district. The parsed rows are hashed; the page is stored in R2 only when they change.
- A project's work table costs one more post. These are read for projects that are new or whose list figures moved, plus the ten longest-unchecked each night, one at a time with a one-second pause. A first load is 141 requests; a normal night is 11.
- Only the detail panel (a few KB) is kept as evidence for a work table, not the whole 105 KB page.
- `POST /admin/ingest?source=status&details=N` runs it by hand, capped at N work tables.

**How the two sources are joined** (`kiifb_status::link`)

The status page and the map dashboard share no identifier. A status "project" is a dashboard sub-project, and its "works" are the dashboard's packages.

- Join when approved amount equals the sub-project estimate to the rupee, the department is the same, and the implementing agency is the same, and exactly one record fits on each side.
- If the agency differs, join only when the names share at least half their distinctive words.
- If more than one record fits, do not join.
- A work joins a package only when the published titles are identical after removing case and punctuation.

Result on 30 Sep 2026: 53 of 140 status projects joined, covering 204 of the 306 map packages; 144 works joined to a package. Every joined project but three also had a work title match, which is independent confirmation.

**Decisions**

| Decision | Why |
|---|---|
| Separate tables (`funding_projects`, `funding_works`, `funding_observations`) | The source has its own identity and history; unjoined projects still get a page |
| Zero approval is stored as "not approved yet"; zero paid is stored as zero | "Under evaluation" projects show 0 approved; a work with nothing paid is a real state |
| "Paid above approval" needs more than 1% | The first version fired on a ₹1 difference caused by rounding paise |
| No flag rule yet for paid above approval | Flags hang off map projects and need a published rule version; it is shown as a plain statement for now |
| Totals are labelled as not Ernakulam's alone | School clusters and similar projects span districts |

**Pages:** `/funding` (list, four sort orders), `/f/{ref}` (one project and its works), a section on `/p/{code}` for joined packages, a block on the front page, `/api/v1/funding` and `/api/v1/funding.csv`.

## Build 4: who must repair it (PER-850)

A third source: the Kerala PWD defect-liability (DLP) list, `pwd.kerala.gov.in/IMF_website/Projects/wings_list.php`.

**What it adds.** Finished PWD works whose contractor must still repair defects: work, contractor, liability dates, division. 199 distinct works for the Ernakulam-area divisions on 30 Sep 2026, 46 of them ending within 90 days.

**How it is read** (`crates/worker/src/liability.rs`)

- One request per wing finds its divisions. Each division named after Ernakulam, Aluva or Muvattupuzha is then read page by page through the site's own pager links. 22 requests in all, a second apart.
- Weekly. The nightly trigger skips it unless the last read is over six and a half days old. `POST /admin/ingest?source=liability` forces a read.
- PWD spells divisions several ways ("Buildings Division Ernakulam", "Buildings Division, Ernakulam") and each spelling holds different works, so every spelling is read.

**Decisions**

| Decision | Why |
|---|---|
| Phone numbers are never parsed into a record | The pages print contractors' and officers' numbers; they are personal data we have no need for |
| The stored copy is our extract (TSV), not the page | A stored page would republish those numbers |
| The agreed contract amount is read, though PWD has commented it out in its markup | Decided by the owner on 30 Sep 2026: it is the public cost of a public work. The page and the methodology say that PWD's own page does not display it. PWD has filled it in for only 2 of the 199 Ernakulam works, so it appears on those rows and in the downloads, not as a headline total |
| The contractor's address, commented out the same way, is not read | Personal data we have no need for |
| A work's identity is wing + name + start date + contractor | PWD gives no id. About a fifth of its rows repeat a work, sometimes with the date written differently |
| Contractors are grouped by a key that ignores titles, case and punctuation | "Shri. P.V. Stephan" and "P V STEPHAN" are one contractor; two people with one name can collide, and the page says so |
| The division link is written with raw base64, as the site's pager writes it | The site answers "Invalid Page URL" to a percent-encoded value |
| No top-level nav item | Five already fill a phone's width; the page is linked from the front page and footer |

**Pages:** `/liability` (filters by wing and contractor, soonest to end first), a block on the front page, `/api/v1/liability` and `/api/v1/liability.csv`.

## Build 5: a correction, the whole state, and the rest of the buildable list (30 Sep 2026)

### A correction to build 3

KIIFB's list states "Payment Released" per project. For every Ernakulam project with works, that figure is an exact whole multiple of what the works add up to: 1 for most, and 2, 3, 4, 6 or 14 for projects filed under several districts. The list counts the same payments once per district. Build 3 showed the listed figure, so the total read ₹9,567 crore where the works add up to ₹5,608 crore.

Paid is now the sum of the works' paid amounts. The listed figure is kept (`released_listed`) and shown next to it on a project's page wherever the two differ, with the multiple. A project whose work table has not been read yet still shows the listed figure, marked provisional. Changes in the listed figure are no longer recorded as changes.

### What was added

| Feature | Where | Notes |
|---|---|---|
| Run log, alerts, health | `runs.rs`, `/status`, `/api/v1/status` | Optional `ALERT_WEBHOOK` secret; 503 when a source is stale |
| Visit counts | `page_views` table | Per kind of page, language and day; robots skipped by User-Agent |
| Whole state | `PILOT_DISTRICT = "*"` | 3,622 dashboard projects, 1,404 status projects, 2,475 PWD works |
| District pages | `/d/{district}` | Same template as the front page, narrowed |
| Contractors | `/contractors`, `/c/{key}` | One key per contractor across the sources (`entity.rs`) |
| Agencies | `/agencies`, `/a/{key}` | Abbreviations resolved to full names |
| Rule version 2 | `flags.rs` | Overdue ignores ended works; new "paid above approval" on status-page works |
| Malayalam search | `search.rs` | Dictionary of places and project words, spelled-out fallback |
| Share images | `og.rs`, `/og/...png` | Drawn in the Worker with the site's fonts; Latin text only |
| Offline | `assets/sw.js` | Opened pages are kept; the first script on ordinary pages |
| Accessibility pass | templates, `app.css` | See below |
| CI | `.github/workflows/ci.yml` | Tests and a wasm compile check |

### Decisions

| Decision | Why |
|---|---|
| A project with no district from KIIFB is "district not stated" | 393 pins carry no district. Guessing from coordinates against simplified borders would misplace border cases |
| The state's front page has district tiles, not 140 constituency tiles | Constituencies appear once a district is chosen |
| Thousands of map dots are two SVG paths of zero-length strokes | One element per dot made the front page three times larger |
| The status page is read district by district (14 requests) | It is the only way to learn which districts a project is filed under |
| Work tables are read 120 a night | A first statewide load is about 1,400 requests; spreading it over twelve nights is kinder to KIIFB than one burst, and keeps a run inside the limit on requests |
| A PWD work's district is its office's district | PWD states no location; a division can cover neighbouring districts, and the page says so |
| Ordinary pages now load one script | Offline support needs a service worker, and that needs registering |
| Share images carry English text only | Malayalam needs a text shaper the Worker does not have; KIIFB's titles are English |
| Dashboard contractor names are re-spaced only where capitals show the joins | KIIFB runs names together; an all-capital name has no joins to find |

### Accessibility pass

Checked on every kind of page by script: one `h1`, no skipped heading levels, every link, button and form control named, no duplicate ids, labelled landmarks, language set. Fixed: the focus ring was blue and vanished on the green hero (now ink, blue on black surfaces); sideways-scrolling tables can be reached and scrolled by keyboard and have column headers marked; small link targets raised to 24 px; the map page says its pins need a pointer and that the list has the same projects. Contrast of every text and surface pair in use is 6:1 or better.

Not done: a pass with a real screen reader, and a keyboard-operable map.
