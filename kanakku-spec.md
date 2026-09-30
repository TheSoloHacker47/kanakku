# Kanakku: Public Project Accountability Platform for Kerala

*Working name. "Kanakku" (കണക്ക്) means "accounts" in Malayalam, as in "give account of".*

## 1. What it is

A mobile-first, Malayalam-first web app that shows citizens where public money for a project was sanctioned, who got the contract, how much has been paid, and how far the work has actually progressed. Where public data is missing, it helps citizens file precise RTI applications, tracks the replies, and publishes verified documents back onto the project page.

**Pilot scope:** Ernakulam district. KIIFB projects plus LSG (panchayat and municipality) plan projects, joined with their e-tender awards.

### Core principles

- **Every number links to its source.** That source is a government portal snapshot, an RTI reply PDF, or a verified citizen photo.
- **Facts and flags, never accusations.** Say "14 months past scheduled completion", never "corrupt".
- **Strictly non-partisan.** Apply the same rules to every party, MLA, and local body.
- **Citizens file RTIs themselves.** The platform drafts, tracks, and verifies, but never files on anyone's behalf.

## 2. Users

| User | What they do |
|---|---|
| Resident | Browses projects near them, follows a project, uploads site photos |
| RTI filer | Picks a gap, gets a draft, files it on the Kerala RTI portal, uploads the reply |
| Moderator | Reviews AI-extracted facts from RTI replies and photos before they go live |
| Journalist / researcher | Filters by flags, exports data (CSV, JSON API) |
| Admin | Manages sources, scrapers, and the flag rules |

## 3. Data sources

| Source | What it gives | Access method | Notes |
|---|---|---|---|
| KIIFB project status + GIS dashboard (kiifb.org, gis.kiifb.org) | Project list, sanctioned amount, status, location | Scrape | JSP pages, may need a headless browser |
| KIIDC and other SPV project pages | Scheduled completion, AS amount, progress | Scrape | Format varies per agency |
| Sulekha (plan.lsgkerala.gov.in) | LSG plan projects, allocation, expenditure | Scrape | Check which reports are public without a login |
| eTenders Kerala (etenders.kerala.gov.in) | Tenders, Award of Contract (AOC), bidder count | Scrape | NIC GePNIC search pages often have a captcha, so try the CPPP AOC listings first |
| CPPP (eprocure.gov.in) | AOC records that aggregate state portals | Scrape | A large public dataset has already been built from this, so it's a proven path |
| MPLADS eSAKSHI | MP fund works, geo-tagged photos | Scrape / dashboard | Data starts 2023-24 |
| kerala.data.gov.in | Misc datasets | API / download | GODL license: attribute the source |
| CAG / AG Kerala audit reports | Audit findings per project/department | PDF + AI extraction | Link audit paras to projects |
| RTI replies (user uploads) | Bills, MBs, EoT orders, completion certs | Upload + AI extraction + human review | The richest layer |
| Citizen photos | Ground truth on physical progress | Upload (geo-tagged) | Verified against project coordinates |

## 4. Features

### 4.1 Project map and list
- Map of Ernakulam showing project pins, coloured by flag status.
- Filters: department, local body, assembly constituency, flag type, funding source, status.
- Search in Malayalam and English, including transliterated names ("Aluva bypass" / "ആലുവ ബൈപാസ്").

### 4.2 Project page
- **Money trail:** Budget/sanction, then tender, then contract, then payments, then progress.
- **Timeline:** Scheduled vs actual dates, including every extension-of-time order.
- **Flags** with a plain explanation of each and the data behind it.
- **Documents:** Every source snapshot and RTI reply, with the date retrieved.
- **Gaps:** "What we don't know yet", with a one-tap "Help find this out" button leading to the RTI flow.
- **Photos:** A citizen photo timeline showing date, distance from the site, and verification status.
- Follow button: notify me when something changes.

### 4.3 Flags (rule-based, transparent, versioned)

| Flag | Rule (v1) |
|---|---|
| Overdue | Today > scheduled completion and status ≠ completed |
| Cost escalation | Latest revised amount / original sanctioned amount ≥ 1.2 |
| Low competition | Bids received ≤ 2, or a single-bid award |
| Stale | No official progress update for 90+ days |
| Payment vs progress mismatch | Paid % exceeds reported physical % by 25+ points |
| Photo contradiction | Verified photos show no work while the official status says "in progress" or "completed" |
| Audit mention | Project appears in a CAG/AG audit paragraph |

The flag rules are published on a public methodology page. Each flag stores the rule version that produced it.

### 4.4 RTI Centre (AI-assisted)

1. **Gap detection.** Each project is checked against a standard document checklist:
   - Administrative sanction (AS) order
   - Technical sanction (TS)
   - DPR / estimate
   - Tender comparative statement
   - Agreement
   - Measurement book extracts and bills paid
   - Extension-of-time orders
   - Quality inspection reports
   - Completion certificate

   Before suggesting an RTI, it checks whether the document is already in the department's Section 4 proactive disclosures.
2. **Draft generation.** Claude drafts a narrow, document-focused RTI in Malayalam or English. The draft is addressed to the correct public authority, pre-filled with the project code, AS number and dates, and has numbered questions. It avoids "why" questions and bulk requests, both of which invite Section 7(9) refusals.
3. **Dedup.** If someone has already requested the same document from the same office, the user sees "Already requested, reply due {date}" and can follow that request instead.
4. **Filing.** The user files on rtiportal.kerala.gov.in in their own name (₹10 fee, BPL exempt), then pastes back the registration number.
5. **Deadline tracking.** Day 25: a reminder. Day 31 with no reply (deemed refusal): the user gets a pre-drafted first appeal, which is free and can be filed online.
6. **Reply upload.** The user uploads the PDF. AI extracts the facts, compares them with existing data, and highlights mismatches. The upload then enters the moderation queue.
7. **Publish.** A moderator approves it. The page updates with the redacted document linked beside every number derived from it.

### 4.5 Citizen photos
- Taken through the in-app camera, which is preferred over gallery uploads because it captures live GPS and a timestamp.
- Auto-verification: the photo must be within 300 m of the project location and have a plausible timestamp.
- EXIF data is stripped before the photo is shown publicly. The original is kept privately for verification.
- Moderators can mark a photo as "shows work in progress", "no visible work", or "unclear".

### 4.6 Moderation
- A queue for RTI extractions, photos, and user-reported corrections.
- Two-person approval for anything that creates or clears a flag.
- A full audit log of who changed what and why.

### 4.7 Open data
- Public read-only JSON API and CSV exports, published under an open license with attribution to the original sources.

## 5. Cloudflare stack

```
                ┌───────────────────────────────┐
  Browser/PWA → │ Workers (static assets + API) │ ← Turnstile
                │  React SPA  +  Hono API       │
                └───┬──────┬──────┬──────┬──────┘
                    │      │      │      │
                   D1     R2     KV   Queues ──► consumer Workers
                (core DB) (docs, (cache,       (scrape, extract,
                          photos, flags)        notify)
                          snapshots)
                    │
        Workflows (RTI lifecycle, 30-day timers)
        Cron Triggers (nightly scrapes, flag recompute)
        Browser Rendering (JS-heavy gov portals)
        AI Gateway → Claude API (drafting, extraction)
        Workers AI + Vectorize (embeddings, project matching)
        Access (moderator/admin area)
```

| Concern | Choice | Why |
|---|---|---|
| Frontend | React + Vite + Tailwind + TanStack Query, served as Workers static assets | Familiar stack; one deploy target |
| PWA | Service worker, installable, offline project pages | Many users will be on patchy mobile data |
| API | Hono on Workers | Small, fast, typed routes |
| Main DB | D1 (SQLite) with FTS5 for search | One district fits comfortably within D1 limits. Shard by district later, or move to Hyperdrive + Postgres if needed |
| Files | R2: `snapshots/`, `rti/original/`, `rti/redacted/`, `photos/original/`, `photos/public/` | Cheap, no egress fees |
| Cache | KV for rendered project summaries and map tile metadata | Fast reads for hot pages |
| Background jobs | Queues: `scrape-jobs`, `extract-jobs`, `notify-jobs` | Retries, backpressure, isolation |
| Scheduling | Cron Triggers: nightly source scrapes, hourly flag recompute | Built in |
| Long-running processes | Workflows for the RTI lifecycle (filed → sleep 25d → remind → sleep 6d → appeal prompt) | Durable timers across weeks |
| Scraping | Workers `fetch` + Cheerio-style parsing. Browser Rendering for JSP/JS pages | Stays inside Cloudflare |
| Scraping fallback | Scrapling on a small India-based VPS or home machine that pushes to a signed `/ingest` endpoint | Some gov portals block non-Indian or datacenter IPs |
| LLM | Claude API through AI Gateway | Strong Malayalam and document extraction; the gateway adds caching, logs, and rate limits |
| Embeddings / matching | Workers AI embeddings + Vectorize | Match "the same project" across KIIFB, Sulekha, and tender titles |
| Maps | MapLibre GL + Protomaps PMTiles hosted on R2 | No per-request map API bill |
| Auth (citizens) | Email or phone OTP (Resend for email), sessions stored in D1 | Low friction; no social login needed |
| Auth (moderators) | Cloudflare Access in front of `/admin` | Zero-trust without custom code |
| Anti-abuse | Turnstile on uploads and sign-up, plus rate limiting rules | Uploads are the main attack surface |
| Analytics | PostHog (cookieless mode) | Product analytics without tracking citizens |
| Email/notifications | Resend; Web Push later | Deadline reminders and project updates |

## 6. Data model (D1, v1)

```sql
-- where data came from
sources(id, name, base_url, kind, license, last_scraped_at)
snapshots(id, source_id, url, r2_key, sha256, fetched_at)

-- canonical project (one real-world project)
projects(id, title_en, title_ml, department, funding_source, local_body,
         assembly_constituency, district, lat, lng,
         sanctioned_amount, revised_amount, scheduled_start, scheduled_end,
         actual_end, official_status, physical_progress_pct, updated_at)

-- raw records from each source, linked to a canonical project
source_records(id, source_id, snapshot_id, external_id, raw_json,
               project_id NULL, match_confidence, match_status)

tenders(id, project_id, tender_id, ref_no, org_name, published_at,
        closing_at, estimate_amount)
awards(id, tender_id, contractor_name, award_amount, awarded_at,
       bids_received, snapshot_id)
payments(id, project_id, amount, paid_on, bill_no, evidence_doc_id)
time_extensions(id, project_id, order_no, new_end_date, reason, evidence_doc_id)

flags(id, project_id, type, rule_version, value_json, evidence_json,
      status, created_at, cleared_at)

-- RTI
rti_gaps(id, project_id, doc_type, public_authority, status)
rti_requests(id, gap_id, user_id, draft_ml, draft_en, portal_reg_no,
             filed_on, due_on, status, appeal_filed_on, anonymous BOOLEAN)
documents(id, project_id, kind, r2_original_key, r2_redacted_key,
          uploaded_by, rti_request_id NULL, sha256, created_at)
extracted_facts(id, document_id, field, value, confidence,
                mismatch_with_json, review_status, reviewed_by)

-- citizens
users(id, email_or_phone_hash, display_name, created_at)
follows(user_id, project_id)
photos(id, project_id, user_id, r2_original_key, r2_public_key,
       lat, lng, taken_at, distance_m, verification, reviewed_by)

audit_log(id, actor_id, action, entity, entity_id, diff_json, at)
```

## 7. Pipelines

**Scrape** (Cron → `scrape-jobs` queue)
1. Fetch the page, store the raw HTML/PDF in R2 as a snapshot (with a hash).
2. Parse it into `source_records`.
3. Skip it if the hash is unchanged.

**Match** (entity resolution)
1. Normalise names, amounts, dates, and local body.
2. Find candidates using a Vectorize similarity search plus rule scoring (amount within ±5%, same local body, dates overlap).
3. Auto-link above the confidence threshold. Anything in the grey zone goes to the moderator queue.

**Flag**
- An hourly job recomputes all flag rules and writes changes to `flags` and the audit log.
- Followers are notified whenever a flag appears or clears.

**RTI extraction** (`extract-jobs` queue)
1. Run OCR on scanned PDFs (Claude vision, or a text layer when one exists).
2. Extract structured fields (amounts, dates, order numbers) using Claude with a strict JSON schema.
3. Auto-redact the applicant's name and address, plus any personal data.
4. Diff the extracted values against existing values, then send the result to moderation.

## 8. Malayalam and mobile

- UI strings live in i18n files (`ml` default, `en` toggle). Fonts: Noto Sans Malayalam / Manjari.
- Numbers use the Indian format (₹1,23,45,678 and lakh/crore labels).
- Pages are designed for 360px wide screens first, with large tap targets and light bundles (target under 150 KB JS on first load).
- Pages are shareable via WhatsApp, with an OG image per project (generated at the edge) showing its flags.

## 9. Legal, safety, and trust

- **Language:** Only neutral, factual wording. Every claim cites a document. There's a public corrections form with a published response time.
- **Data license:** Attribute GODL datasets as required. Keep scraped snapshots as evidence of what the government published and when.
- **DPDP Act 2023:** Collect minimal personal data, state a clear purpose, allow deletion, and hash phone numbers and emails where possible.
- **Filer safety:** Filers are anonymous publicly by default. Spread RTIs across many filers. Never expose who filed what.
- **Takedown process:** A documented procedure for legal notices. Take down only on a valid order, and log it publicly.
- **Neutrality:** The same flag rules apply to all parties. A methodology page is published before launch.

## 10. Roadmap

| Phase | Scope | Exit criteria |
|---|---|---|
| 0. Spike (1-2 weeks) | Check scrapeability of KIIFB, Sulekha, eTenders/CPPP. Hand-match 20 Ernakulam projects end to end | Know which sources work and how many gaps exist |
| 1. MVP (4-6 weeks) | Scrapers, matching, project pages, flags, map, Malayalam UI | 100+ Ernakulam projects live with flags |
| 2. RTI Centre | Gap detection, drafts, tracking via Workflows, upload + extraction + moderation | First 10 RTI replies published |
| 3. Citizen photos | Camera upload, geo-verification, photo timeline | 50 verified photos |
| 4. Scale | More districts, open API, partner outreach (CivicDataLab, journalists, RTI groups) | 3 districts live |

## 11. Open questions and risks

- Which Sulekha reports are publicly viewable without a login?
- Is there captcha or IP blocking on eTenders and KIIFB from Cloudflare egress? (This decides whether the India VPS fallback is needed.)
- How reliable is project matching across sources, given there's no shared ID? (Manual moderation cost could be high at first.)
- How are PIOs responding? The rate of useful RTI replies will decide whether the RTI Centre is the core of the product.
- Funding and sustainability: grants (for example, open-government funders), donations, or a paid API for media and research. Avoid anything that looks partisan.
- Moderator capacity: who reviews the uploads? (Could start with volunteers, then journalism students.)
