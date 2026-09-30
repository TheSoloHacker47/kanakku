# Tender reachability spike (PER-820)

Run on 30 Sep 2026. Question: can we read tender and award data for Kerala, and from where?

## Answer

- **Awards and bid counts: no-go.** eTenders Kerala and CPPP both answer from Cloudflare, but every page that lists results sits behind a captcha. We do not solve captchas, so those pages are out.
- **Payments per work: go.** `kiifb.org/prjStatus.jsp` publishes, without a captcha or login, every KIIFB project with its approved amount and payment released, and a per-work breakdown. This was not in the plan and is the most useful find.
- **Contractor names for finished PWD works: go.** The PWD defect-liability list names the contractor and the liability period for about 2,980 works.

## Reachability

Each URL was fetched once, with the User-Agent `KanakkuBot/0.1 (civic transparency research)`.

| Source | From the dev machine (India, Chennai edge) | From a Cloudflare Worker (Singapore edge) |
|---|---|---|
| `etenders.kerala.gov.in` | TCP connect times out | 200 |
| `eprocure.gov.in/cppp` | TCP connect times out | 200 |
| `kerala.data.gov.in` | TCP connect times out | 521 (origin down) |
| `gepnicreports.gov.in` | times out | 522 |
| `thottariyaampwd.kerala.gov.in` (PWD project system) | times out | 522 |
| `www.kiifb.org` | 200 | not tested |
| `tender.lsgkerala.gov.in` | 200 | 200 |
| `pwd.kerala.gov.in` | 200 | not tested |

The dev machine cannot open a TCP connection to several NIC-hosted addresses (164.100.x.x), though DNS resolves. Cloudflare can. So the nightly ingest would work, but local development against those hosts needs saved fixtures.

The Worker test used `wrangler dev --remote` with a throwaway script; nothing was deployed.

## Source by source

### eTenders Kerala (NIC GePNIC): no-go

Captcha on: Results of Tenders, Tender Status, Active Tenders, Tenders in Archive, Advanced Search.

"Tenders by Organisation" shows a captcha and the note "Provide Captcha and click on Search button to list By Organisation", yet the same response already contains the organisation table (129 organisations with counts) and working links to each organisation's active tenders. One link was followed once to confirm this. It should not be built on: the page states that a captcha is intended, and it lists only active tenders, not awards.

### CPPP: no-go

`/cppp/resultoftendersnew/cpppdata` is a captcha-gated search form and returns no rows without it.

### kiifb.org project status: go

`https://www.kiifb.org/prjStatus.jsp`, one GET, 758 KB.

- 1,411 projects statewide: department, project name, SPV, approved amount, payment released, status (1,160 approved, 251 under evaluation).
- Totals on the day: about ₹93,287 crore approved and ₹87,591 crore released.
- The public form filters by district. Ernakulam (`optDistrict=7`) returns 140 projects in 103 KB.
- Selecting a row posts the same form with `hidSelProject=<id>` and returns a "Work Information" table: work name, SPV, approved amount, paid amount, status ("Work Awarded" or "Work Started").

In a sample of 30 Ernakulam projects there were 137 works, ₹3,284 crore approved and ₹2,421 crore paid. Two works showed more paid than approved.

Limits:

- No contractor names, dates, or bid counts.
- No shared key with the map dashboard. Ids here are opaque (`f4ue5vh2sj0q`); the dashboard uses codes like `PWD016-05-01`. A plain title comparison linked 12 of the 30 sampled projects to a record already on the site. Proper matching is needed (PER-831).
- The work table's HTML is malformed (rows are not closed), so the parser must split on row starts.
- A full Ernakulam refresh is 1 list request plus 140 detail requests of about 105 KB each.

Fixtures: `crates/core/tests/fixtures/kiifb_status_ernakulam.html`, `kiifb_status_detail.html`.

### PWD defect-liability list: go

`https://www.pwd.kerala.gov.in/IMF_website/Projects/wings_list.php` links one listing per wing: Roads 392, Buildings 2,549, Bridges 39, NH 2, RICK 2.

Columns: work name, agreed amount (often blank), liability start and end dates, contractor name, PWD division, subdivision, section, and officers' contact numbers. No captcha. There is an Excel export.

This covers completed works only. It tells a resident who must repair a road or building, and until when.

### LSGD tender notices: partial

`tender.lsgkerala.gov.in` lists tender notices for every local body, with a district filter and an RSS feed (445 KB) linking each notice PDF. Notices only, no awards. Useful later for the LSG milestone.

### Not reachable or not useful

- KIIFB Bill Tracking (`status.kiifb.org`): for contractors, by mobile number and OTP. Out.
- KIIFB PMAS (`projectupdates.kiifb.org`): login. Out.
- PWD project system (`thottariyaampwd.kerala.gov.in`): timed out from both places on the day. Retry later.
- KRFB and RBDCK sites: reachable, only glanced at. They publish project lists and tender notices; not checked for awards.

## What this means for the roadmap

1. Milestone 2 as written (tender schema, matching, low-competition flag) cannot be built from scraping. Who won and how many bid is only available behind captchas.
2. The route to bid counts is RTI: ask the tendering authority for the comparative statement. That fits the RTI Centre, and it makes a strong first use of it.
3. The next build should ingest the kiifb.org status page. It adds "approved against paid" for each work to projects that today show only an amount and a status.
4. The PWD liability list is a second, smaller addition with contractor names.

## Alternatives for winners and bid counts (researched 30 Sep 2026)

The data exists. An aggregator's public page for one KRFB tender (`2026_KRFB_840628_1`) lists the documents the government portal holds for an award: bidder details, BOQ comparative chart, financial bid opening summary, technical evaluation summary. That tender's title carries the KIIFB code (`PWD016-34`), which would make matching to our projects straightforward.

| Route | Gives | Cost and speed | Verdict |
|---|---|---|---|
| A person enters the captcha on eTenders Kerala, searches, and saves the result pages; we parse the saved pages | Winner, award value, bidder list, bid count | Free; a few hours for Ernakulam; repeat when new awards appear | Recommended to start |
| One RTI per implementing agency (KRFB, KIIDC, KITCO, KWA and others) asking for a list of KIIFB tenders in Ernakulam with bids received, bidders, and award value | The same, in bulk, as an official record | ₹10 each; up to 30 days; reply quality varies | Recommended, file early |
| Written request to the portal's operators (Kerala State IT Mission, NIC) for a data feed or bulk export | Everything, ongoing | Free; slow and uncertain | Worth one letter |
| Licence from a tender aggregator (BidAssist, Tenderkart, Tender247) | Winners and bidder lists, ongoing | Paid; needs written permission to republish; terms not reviewed | Possible later |
| Sources already open: map dashboard (12 of 306 projects name a contractor), PWD liability list, KIIFB newsletters, Assembly answers, CAG reports | Winner name for some works; no bid counts | Free | Partial |
| GeM bid results | Public seller lists and ranks | Free | Not relevant: construction works are not tendered on GeM |
| Open datasets from civic groups | — | — | Found one for Assam, none for Kerala |

Ruled out: captcha-solving services or OCR, using the portal's mobile-app interface to avoid the captcha, and copying paywalled aggregator data.

Not verified: what exactly the eTenders results pages show after the captcha (inferred from the aggregator's document list), and whether the GePNIC reports site has anything useful (it timed out again on retry, as did the PWD project system).
