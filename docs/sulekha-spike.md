# Spike: what Sulekha publishes without a login (PER-826)

2 October 2026. About fifteen requests in total, two seconds apart, with an identifying User-Agent. No login, no captcha, nothing submitted except the public form's own choices.

## Answer

**Yes: every local-body plan project is public, with its planned and spent amounts and a Malayalam name.** It is the largest source Kanakku could add, and the one closest to people's own streets.

## Where it is

`https://plan.lsgkerala.gov.in/formulation/Public.aspx`, linked from Sulekha's home page as the public view. It is an ASP.NET WebForms page, so every step is a POST carrying the page's `__VIEWSTATE`, not a URL.

| Step | Choice (form value) | What comes back |
|---|---|---|
| 1 | Year (`drpYear`: `29` = 2025-26, back to 2012-13) | – |
| 2 | Local-body type (`drpType`: 1 District Panchayat, 2 Block, 3 Municipality, 4 Corporation, 5 Grama Panchayat) | One row per district: number of local bodies, number of projects, planned amounts by sector (productive, service, infrastructure) in lakh |
| 3 | A district (`gvState`, `Select$n`) | One row per local body, same columns (Ernakulam: 82 grama panchayats) |
| 4 | A local body (`gvStat`, `Select$n`) | **Project list** (`gvProjects`), 20 to a page: number, **project name in Malayalam**, **planned amount** ("Formulation") and **amount spent** ("Expense"), both in rupees |
| 5 | A project (`gvProjects`, `Select$n`) | A **13-page PDF** (Crystal Reports): project number (e.g. `S0002/26`), English name, sector and sub-sector codes, implementing officer (e.g. Secretary, GP), mode (direct or through an agency), and more on later pages |

Example: Alangad Grama Panchayat, Ernakulam, 2025-26 has 219 projects on 11 pages. Row 2 reads "പദ്ധതി നിർവഹണ മോണിട്ടറിംഗ് ചെലവുകൾ", planned ₹5,00,000, spent ₹3,21,848.

## Size

Grama panchayats alone in 2025-26: Thiruvananthapuram 19,130 projects, Kollam 17,846, Kottayam 18,247, Alappuzha 15,816, Pathanamthitta 12,582. The statewide total across all local-body types is likely 250,000 to 300,000 projects a year.

Reading every list page once is roughly 14,000 requests of about 80 KB each (the view state is heavy), about 1 GB. At one request every two seconds that is about 8 hours, so it has to be spread over nights.

## Things to know

- **Malayalam names, in Unicode, on the list.** That makes it a natural fit for a Malayalam-first site, and the search already handles Malayalam.
- **No location** on the list: no ward and no coordinates. These pages cannot be mapped. The PDF may carry the ward on a later page; that needs checking.
- **The PDFs are heavy** (13 pages each) and their Malayalam text extracts badly: the font has no proper character map, so vowel signs are lost. The English fields extract cleanly. Read PDFs only on demand, never in bulk.
- **robots.txt** disallows URLs with query strings (`/*?`), `.php` and `.jsp`. The public form is a POST to a plain `.aspx` address, so it is not covered by those lines. The intent is still clear: they do not want bulk crawling of dynamic pages.
- **Who runs it:** Information Kerala Mission (IKM), `webmaster@ikm.gov.in`, for the Local Self Government Department.

## Recommendation

1. **Ask first.** Write to IKM (and copy LSGD) describing Kanakku and asking for a bulk export or permission to read the public view slowly. The volume is real (14,000 requests a year), and a civic site that asks is far harder to block or criticise.
2. **Meanwhile, a one-district pilot:** Ernakulam's 82 grama panchayats (about 900 list pages), read over a week at one request every two seconds. Build the pages (local body → projects; planned vs spent; a "nothing spent yet" view late in the year) and judge whether people use them.
3. **Statewide only after IKM answers,** or after a few weeks without objection to the pilot, using a Cloudflare Queue so the work spreads across many short invocations. A single cron run is capped at 15 minutes.
4. **No new flags at first.** Planned vs spent is meaningful only against the financial-year calendar. Show it plainly, and design flags later with care.

## Built, waiting to be switched on (2 October 2026)

The reader is deployed and off. What exists:

- `kanakku_core::aspnet`: reads an ASP.NET form's hidden state and drop-downs, builds a postback, and reads GridView tables, including the pager. Tested against a saved copy of the district-panchayat summary.
- `kanakku_core::sulekha`: the summary tables (`gvState`, `gvStat`) and the project list (`gvProjects`); the read refuses a table that has lost its Formulation or Expense column.
- `crates/worker/src/sulekha.rs`: the walk (year → kind → district → each local body → each page), one request every two seconds with the scraper's User-Agent, a nine-minute time box on its own 04:00 IST cron, and the position saved in `crawl_state` after every local body, so the next night carries on. After a full pass it rests seven days.
- Migration 0015: `local_bodies`, `plan_projects` (planned and spent per project, with `first_seen_on`, `changed_on`, `missing_since`) and `crawl_state`. Each local body's list is kept as a TSV extract in R2, not the raw pages, which are mostly view state.

**To switch on the Ernakulam pilot** (only once IKM agrees, or the user decides after 16 October):

1. Set `SULEKHA = "Ernakulam:gp"` in `wrangler.toml` and deploy.
2. Run it once by hand and watch it: `POST /admin/ingest?source=sulekha` with the ingest token.
3. The project-list parser has been tested only on a list built to the spike's description, because the spike saved no list page. Expect to adjust column matching on the first real run; the read refuses rather than storing a misread page.

Pages (a local body's projects, planned against spent) come after the first real data, so they are designed around what Sulekha actually returns.
