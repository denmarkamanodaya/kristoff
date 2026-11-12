# Phase 7: add OmniCorpus codes and Supreme Court decisions

The owner approved this on 2026-11-12 (decision D-19, scope "Both"). The evidence is in
[`../datasets/omnicorpus-evaluation.md`](../datasets/omnicorpus-evaluation.md).

## What gets added

| Set | From OmniCorpus @ bb0626ce | Into | Shown as |
|---|---|---|---|
| Codes and old statutes: RPC (Act 3815), Family Code (EO 209 / EO 227), Labor Code (PD 442), Rules of Court (1997 civil, 2000 criminal, 1989 evidence, the 2019 A.M.s), plus the 1,419 RAs we lack | statute rows | `phlaws.db`, through a new `sources` table | law cards. Every code provision carries the status label **`[As enacted]`**, which the web's existing status badge renders |
| Supreme Court decisions, 1996 to 2024-04 (31,180) | decision rows | new `phcases.db` | a new **case** card: "G.R. No. … · short title (year)", with a link to the E-Library |

Where both datasets hold the same RA, our current dataset wins (it is newer). OmniCorpus only
fills gaps.

## Rules for answers (prompt, desktop-only addition)

- **Codes.** A provision labelled `[As enacted]` is the original text. The answer says so, and
  points to amending RAs found with `search_ph_laws` (for example RA 8353 for rape, RA 10951 for
  RPC fines, and the renumbered Labor Code). It never states a current article number or fine from
  memory.
- **Decisions.** Cite by G.R. number. Quote only from tool results. Name parties as the decision
  does (the Court's own pseudonyms, AAA/XXX, are kept).

## Design

- **Ingest** (`kristoff-ingest`):
  - An OmniCorpus reader, with column names in `columns.rs`.
  - A cleaner that strips the E-Library footer and OG/NAR headers and fixes mojibake.
  - A splitter for rows that have no newlines: article/section heads, checked against known
    counts (RPC 367 articles).
  - Dedupe against existing RAs.
  - Decisions → `phcases.db`:
    - Metadata (case id, G.R. no., date, ponente, title, url).
    - Full-text FTS5 over 480-token passages.
    - One e5 vector per decision, from its first 480 tokens.
- **Search** (`kristoff-law`):
  - `LawHit` gains source and edition fields.
  - A new `CaseIndex` implements a new core port, `CaseSearch`.
- **Core port `CaseSearch`:**
  - `warm()`
  - `case_count()`
  - `search(query, filters{year_from, year_to}, limit ≤ 8) -> Vec<CaseHit>`
  - `get_case(case_id_or_gr, focus: Option<&str>) -> Option<CaseView>`, returning up to 3
    focused passages
  - Each method has a fake and a contract suite.
- **Tools** (defined once in kristoff-contracts):
  - `search_ph_cases {query, year_from?, year_to?, limit?}`
  - `get_case {case, focus?}`
  - The shared cap of 3 tool calls per question covers all four tools.
  - Results go out as `search_result` blocks.
- **Contract (additive):**
  - Citation `kind: "case"` (`docTitle` = display title, `sourceUrl` = E-Library URL, page null).
  - `config.features.cases` and `caseCount`.
  - `lawReady` waits for both indexes.
- **Bundle:** `phcases.db` is added to `tauri.bundle.conf.json`. `desktop/data/omnicorpus.lock`
  holds the revision and SHA-256, and `build-dmg.sh` verifies it. The expected DMG is about
  1.1 GB.
- **Attribution:** ODC-By credit (Ramos Jr., 2024) goes in THIRD_PARTY_NOTICES.

## Exit

- **ci-local** is green, including the real-db contracts for both indexes.
- **Smoke** passes: RPC Art. 335 (as enacted), Family Code Art. 36, Labor Code Art. 282 (old
  numbering), and 5 G.R. look-ups.
- **Eval:** the existing 40 questions plus about 15 new code and case questions; the existing
  bars hold.
- **Release app:** one answer shows a law card with `[As enacted]` and a case card.
- **DMG:** rebuilt from `~/.kristoff-anthropic-key`, then the owner tests it and re-sends it to
  the brother.
