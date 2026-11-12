# OmniCorpus evaluation (Phase A)

Date: 2026-11-12. Plan: `~/.claude/plans/lets-use-plan-to-serene-stonebraker.md`, Phase A.

- **Dataset evaluated:** `mongramosjr/philippine-omnicorpus` @ `bb0626ce527b159a32a6baefabee0ea1e0e01d2f`, one file
  `data/philippine_laws.parquet` (654,080,549 bytes, SHA-256
  `326b5d1bd4fa49ead5e4ebed1b9b0512054262297795a5a105ab60c3086075ba`, verified).
- **Compared against:** the current index `desktop/data/phlaws.db`, built from
  `endomorphosis/ipfs_philippines_laws` @ `66ffcff9`.
- **Method:** read-only; Python `-I` in a throwaway uv venv (pyarrow + polars); `phlaws.db`
  opened `immutable=1`. The scripts live outside the repo.

## Summary

OmniCorpus fills every statute gap we have, but the codes are the original texts as enacted,
not consolidated with later amendments:

- RPC: 1930 text.
- Labor Code: 1974 text, old numbering.
- Family Code: 1987 text.
- Rules of Court: the 1997/2000 base rules plus the two 2019 amendment orders as separate
  documents.

It also holds 31,180 full-text Supreme Court decisions (1996 to April 2024). It has no media,
culture or history rows, despite the card.

Its text is complete per document, but it has no structure: no newlines at all, headings glued to
the text, and mojibake in about half the rows. Adding it needs a new splitter and a text fixer,
not just a new column map.

**Recommendation:** add the statutes and codes now, as a targeted subset with edition labels. Do
not put decisions in the DMG; ship them later as an optional download.

## 1. Schema and row counts

The columns are:

| Column | Notes |
|---|---|
| `text` | Full document text |
| `citation_information` | A JSON string |
| `citations` | A list; empty in all 91,293 rows |
| `url` | Every row comes from `elibrary.judiciary.gov.ph` |
| `label` | The E-Library shelf |

The type field is `citation_information.type`. Its keys depend on the type:

- Decisions: `case_name`, `citation`, `court`, `date`.
- Statutes: `short_title`, `title`, `citation`, and `date_of_enactment` or `date_of_issuance`.
- Rules: `date_of_adopted`.
- Constitutions: `date_of_ratification`.

91,293 rows, 1,253.6 M characters:

| type | label | rows | M chars | median chars | p95 chars |
|---|---|---:|---:|---:|---:|
| court_decision | Decisions / Signed Resolutions | 31,180 | 879.4 | 22,157 | 58,205 |
| executive_issuance | National Administrative Register | 19,316 | 102.3 | 2,271 | 20,312 |
| republic_act | Republic Acts | 12,141 | 85.8 | 1,868 | 25,384 |
| executive_issuance | Executive Orders | 5,646 | 24.9 | 2,951 | 11,106 |
| executive_issuance | Presidential Proclamations | 5,349 | 13.4 | 1,277 | 6,132 |
| act | Acts | 4,288 | 24.6 | 2,140 | 18,706 |
| executive_issuance | Administrative Orders | 2,550 | 10.4 | 2,981 | 10,154 |
| other_legal_document | Official Gazette | 2,159 | 7.6 | 841 | 19,098 |
| presidential_decree | Presidential Decree | 2,127 | 16.4 | 3,666 | 22,987 |
| treaty | Bilateral + Regional/Multilateral | 2,064 | 25.2 | | |
| letter_of_instruction | Letter of Instruction | 1,453 | 3.9 | 2,137 | 6,275 |
| batas_pambansa | Batas Pambansa | 890 | 3.4 | 1,250 | 11,516 |
| commonwealth_act | Commonwealth Act | 733 | 5.4 | 2,108 | 28,796 |
| executive_issuance | Memorandum Circulars + Orders | 714 | 2.0 | | |
| other_legal_document | ConCom 1986, ConCon 1934-35, benchbooks, manuals, other | 444 | 45.8 | | |
| letter_of_implementation, general_order | | 223 | 0.8 | | |
| rules_of_court | Rules of Court | 10 | 1.7 | 105,517 | 700,298 |
| constitution | Constitutions | 6 | 0.4 | | |

Other facts:

- 471 rows have duplicate text: 234 issuances, 111 decisions, 45 PDs.
- No text is null or empty.

## 2. Gaps, side by side

| Gap | Current (`phlaws.db`) | OmniCorpus | Edition and completeness |
|---|---|---|---|
| Revised Penal Code (Act 3815) | absent | 1 row, 233,777 chars | **Original 1930 text.** Article heads run to Art. 367, the original count. Rape is still Art. 335; there is no Art. 266-A from RA 8353. Art. 10 still says "a fine not exceeding 200 pesos", so the RA 10951 (2017) fines are not folded in. Complete to "Approved, December 8, 1930" |
| Family Code (EO 209) | absent | 1 row, 127,857 chars (EO 209, titled "The Family Code of the Philippines") | **Original 1987 text** (article heads to Art. 257). Art. 36 (psychological incapacity) is present. EO 227's amendments are a separate 2,056-char row, not merged in |
| Labor Code (PD 442) | absent (only RAs that amend it) | 1 row, 218,988 chars | **Original 1974 text, old numbering.** Art. 260 is "Definitions" and Art. 280 is "Requirements of Registration". There is no Art. 297 and no 2015 DOLE renumbering. The amending PDs (570-A, 1368, 1693…) and BPs (70, 227) are separate rows |
| Rules of Court | absent | 10 rows: 1997 Rules of Civil Procedure (367k chars), Revised Rules of Criminal Procedure 2000 (119k), Revised Rules on Evidence 1989 (56k), A.M. 19-10-20-SC (2019 civil procedure amendments, 135k), A.M. 19-08-15-SC (2019 evidence amendments, 67k), and older rules (1900, 1918, 1964, 1985, 1988) | **Base rules plus amendment orders, not consolidated.** Both 2019 A.M.s are present but titled "Proposed Amendments". The 2019 changes must be read alongside the 1997 and 1989 base texts |
| 1987 Constitution | present (352 articles and sections, structured) | 1 row, 137,918 chars, plus the 1973, 1943, 1935 and Malolos constitutions | Complete ("Ratified: February 2, 1987"). Ours is already structured; no gain |
| Court decisions | absent | 31,180 Supreme Court decisions | Full text, 1996 to 2024 (see §3) |
| Other old statutes | absent | Acts 4,288; Commonwealth Acts 733; Batas Pambansa 890; PDs 2,127; LOIs 1,453 | As enacted |
| Executive issuances | absent | 33,575 (EOs, proclamations, AOs, and the Administrative Register: agency rules) | As issued |

## 3. Court decisions

| Fact | Value |
|---|---|
| Count | 31,180, all from the Supreme Court of the Philippines |
| Years | 1996-01 to 2024-04-15. By decade: 1990s 3,704; 2000s 13,785; 2010s 10,444; 2020s 3,247 |
| Fields | `case_name`, `citation` (26,657 include "G.R. No."; the rest are A.M., A.C. and similar), `date`, `court` |
| Ponente | Not a field. A "SURNAME, J.:" marker appears in the first 3,000 chars of 28,719 decisions |
| Syllabus | None: the text is the full decision. "Syllabus" appears near the top of only 47 |
| Length | median 22,157 chars, p95 58,205, mean 28,203, max 2,364,641 |
| Text head | Phil. Reports page, division, docket, parties, "D E C I S I O N", ponente, body |

Sample head, with names trimmed to initials: `731 PHIL. 339 FIRST DIVISION [ G.R. No. 206770, April 02,
2014 ] PEOPLE OF THE PHILIPPINES, PLAINTIFF-APPELLEE, VS. N. P. AND A. M., ACCUSED-APPELLANTS.
D E C I S I O N R., J.: Before the Court is an appeal from the Decision[1] dated May 30, 2012 of
the Court of Appeals…`

## 4. Overlap with `phlaws.db`

| Fact | Value |
|---|---|
| RA numbers | ours 10,869 (max RA 12330); OmniCorpus 11,825 distinct (max RA 11995; 12,141 RA rows, 223 without a number, such as the Omnibus Rules) |
| In both | 10,406 |
| Only in ours | 463: 302 are newer than RA 11995; 161 are older gaps in OmniCorpus |
| Only in OmniCorpus | 1,419, mostly old RAs (e.g. RA 1, 65, 183, 387…) |
| 20 shared provisions | All 20 found. 17 match OmniCorpus verbatim once case, punctuation and spacing are normalised. The other 3 are near-identical (similarity ratio 0.75–0.96, from punctuation and line-joining differences). There are no amendment differences: both datasets hold the RA as enacted |
| Our truncated rows (16%) | For 30 random `truncated=1` chunks, the tail of our chunk was found in OmniCorpus 20 times. In each of those, OmniCorpus continues the sentence where ours stops (e.g. RA 4730 SEC. 29 ends "…mentioned in"; OmniCorpus goes on "Section 28 hereof and…"). Many of our truncated flags are false positives: headings and signature lines with no closing period |

Is OmniCorpus cleaner? It is **more complete**: each act is one whole row, so nothing is cut at
cross-references. It is **less structured**: there are no section boundaries, and headings are
glued to the text (e.g. `…authority of the same:PRELIMINARY ARTICLE.â This law shall be…`).

## 5. Currency

| Fact | Value |
|---|---|
| Newest RA | RA 11995, 2024-05-22 (ours goes to RA 12330) |
| Newest decision | 2024-04-15 (G.R. No. 232968) |
| RPC amended by RA 10951 (2017)? | **No**: original 1930 fines ("200 pesos") |
| RPC Art. 266-A (RA 8353)? | **No**: rape is still Art. 335. RA 8353 itself is in both datasets as a separate act |
| Labor Code renumbered (DOLE 2015)? | **No**: original 1974 numbering, not even the pre-2015 amended numbering |

## 6. Quality: 30 random non-decision rows (seed 2026)

| Problem | In the 30 | Across all legal rows |
|---|---|---|
| Mojibake (`â`, `Ã`, `Â\x80\x93` for dashes and ñ) | 12 | 22,703 of 60,113 (plus 20,534 decisions) |
| E-Library footer ("Source: Supreme Court E-Library … E-LibCMS") | 30 | 90,928 of 91,293 rows |
| Header before the text (`(NAR) VOL. 11 NO.3 …`, `H. No. 585 / 55 OG No. 20 …`) | 26 | |
| Headings glued to the previous word (no newlines anywhere) | 17 | 0 rows have a newline |
| OCR noise | 1 | rare |
| Cut-off text | 0 (every sampled row ends at a signature or approval line) | 436 legal rows lack the footer, so there may be cut-offs; not checked |
| Duplicates | 0 | 471 duplicate texts |

All the problems are mechanical and fixable at ingest:

- Strip the footer and header.
- Repair mojibake with a latin-1/UTF-8 round trip where it decodes.
- Split on `SEC. n.` / `ART. n.` / `Article n` boundaries.

The splitter is the real work. A plain article-head regex finds 278 of the RPC's 367 heads,
because headings run into the previous sentence.

## 7. Size

These estimates are calibrated on our current db:

- **Chunk size:** about 1,780 text chars per full 480-token chunk, about 1,630 new chars after the
  one-sentence overlap.
- **Table cost:** `chunks` 1.32 B per char, FTS 0.71 B per char, vectors 410 B each.
- **DMG growth:** gzip shrinks the db to 0.45 of its size, which approximates the growth.
- **Ingest rate:** 55 chunks/s on Metal.
- **Today's baseline:** `phlaws.db` 170 MB, DMG 293 MB.

| Set | Rows | M chars | Chunks @480 | Vectors | DB growth | DMG growth | Ingest |
|---|---:|---:|---:|---:|---:|---:|---:|
| RAs we already have (skipped) | 10,495 | 64.0 | 43k | | | | |
| **(a1) codes and old statutes only:** RPC, Family Code, Labor Code, Rules, Acts, CA, BP, PD, LOI, GO, only-OmniCorpus RAs | ~13.4k | 75.7 | ~51k | 21 MB | ~+175 MB | ~+79 MB | ~15 min |
| **(a) all statutes, codes and issuances** (a1 + 33.6k executive issuances) | 44,951 | 224.8 | 156,804 | 64 MB | +521 MB | +234 MB | 48 min |
| (b) decisions in `phcases.db`, embedding the first 480 tokens (1 vector each), FTS over the full text | 31,180 | 875.7 | 31,180 vectors | 13 MB | +1,554 MB | +699 MB | 9 min |
| (b, alternative) decisions fully chunked | | | 550,004 | 226 MB | ~+2.1 GB | ~+0.95 GB | 2.8 h |
| Extras not recommended: treaties, ConCom/ConCon records, manuals, Lawyers' List | 4,667 | 78.3 | 50k | | | | |

## 8. Licence and privacy

- **Licence:** ODC-By 1.0 at dataset level. There is no per-row licence field.
  - 856 rows mention "license" near the top; every sampled one is content (driver's licences,
    agency licences), not usage terms.
  - One row contains "all rights reserved" or "Creative Commons".
  - Each row's source is its `url` (SC E-Library). Attribution goes in THIRD_PARTY_NOTICES and
    the About text.
- **Names:**
  - Decisions carry full real names in `case_name`, `citation` and the text (parties, counsel,
    justices).
  - Pseudonyms appear only where the Court itself used them: 1,504 decisions use AAA/XXX, which
    is 1,203 of 2,599 rape, RA 7610 or RA 9262 decisions. The other half name the parties.
  - Local private use is in line with the dataset's Data Privacy Act notice. Displaying decisions
    in the app shows real names, and question context sent to Claude may include them.

## Recommendation

| Option | What it adds | Size (DB / DMG) | Ingest | Work |
|---|---|---|---|---|
| **1. Statutes and codes only (recommended now): subset a1** | RPC, Family Code, Labor Code, Rules of Court, pre-1987 statutes, 1,419 extra RAs | about +175 MB / +79 MB (DMG about 372 MB) | about 15 min | About 2 days, in kristoff-ingest: <br>• an OmniCorpus reader, with columns in `columns.rs` <br>• header and footer stripping, plus a mojibake fix <br>• a provision splitter for text with no newlines <br>• a `sources` table, and edition labels such as "RPC: original 1930 text, amendments not included; see RA 8353, RA 10951" <br>• new smoke and eval questions |
| 1b. Plus executive issuances (a) | adds 33.6k agency rules, proclamations and EOs | +521 MB / +234 MB | 48 min | the same, plus noise filtering; mostly low value for case questions |
| 2. Codes plus decisions | adds 31k SC decisions in a separate `phcases.db` | about +1.7 GB / about +780 MB on top of option 1 (DMG about 1.1 GB) | about 25 min | Option 1 plus a `CaseSearch` port, fake and contract suite, a `search_ph_cases` tool, citation kind `case`, a UI card, and prompt updates. About 3–4 more days. Better as a **separate optional download** than in the DMG |
| 3. Don't add | nothing | 0 | 0 | 0; the gaps stay (no RPC, Family Code, Labor Code, Rules or decisions) |

The edition risk is the main caveat for options 1 and 2:

- The RPC and the Labor Code are pre-amendment texts. An answer that quotes RPC Art. 335 (rape)
  or 1974 Labor Code numbering without a warning would be wrong in practice.
- **Mitigation, required with option 1:**
  - an edition banner on every card from those sources;
  - prompt text saying these are as-enacted texts;
  - an instruction to also cite the amending acts in our index (RA 8353, RA 10951, RA 10151…);
  - eval questions that check the warning appears.

## Phase 7 ingest decisions (2026-11-12)

This section records what the Phase 7 build actually took. It follows owner ruling D-20
(richest data; size does not matter) and lead decisions made on the same day.

### Statutes, into `phlaws.db`

Every legal row is taken except the exclusions below. The current dataset wins on duplicate RAs.
Each OmniCorpus law carries an edition label, which is shown as its status line:

| Rows | `laws.kind` | Label |
|---|---|---|
| Revised Penal Code (Act 3815), Family Code (EO 209, 1987) and EO 227, Labor Code (PD 442) | ACT / EO / PD | `As enacted` |
| Acts, Commonwealth Acts, Batas Pambansa, PDs, and the RAs the current dataset lacks | ACT / CA / BP / PD / RA | `As enacted` |
| Rules of Court: 1997 civil, 2000 criminal, 1989 evidence, A.M. 19-08-15-SC and A.M. 19-10-20-SC | RULES | `As promulgated` |
| Executive Orders, proclamations, AOs, memorandum orders and circulars, Administrative Register agency rules, LOIs, Letters of Implementation, General Orders | EO / PROC / AO / MO / MC / NAR / LOI / LOIMP / GO | `As issued` |
| Treaties (bilateral, regional, multilateral) | TREATY | `As signed` |
| 1973, 1943, 1935 and Malolos constitutions, the 1934-35 ConCon and 1986 ConCom records, the 1986 draft | CONREC | `Constitutional records` |
| Benchbooks, clerk-of-court manuals, the judicial writing manuals | COURTREF | `Court reference` |

Excluded rows, as logged by the ingest tool:

| Rows | Reason |
|---:|---|
| 2,159 | Official Gazette notices: noise (land, publication and court notices) |
| 250 | IRRs and circulars filed on the Republic Acts shelf, which would collide with real RA numbers |
| 5 | Superseded Rules of Court (1900, 1918, 1964, 1985, 1988) |
| 3 | Not a recognised legal type |
| 1 | Lawyers' List: a personal-data list of lawyers |
| 1 | Election Cases: one 1.4 MB blob of unstructured case lists |
| 1 | The OmniCorpus copy of the 1987 Constitution: already in the current dataset, structured |

Splitter accuracy on the codes:

| Code | Articles found | Note |
|---|---|---|
| RPC | 367 of 367 | |
| Labor Code (1974 numbering) | 341 of 341 | Art. 282 is "Denial of Registration"; the just causes are Art. 321, "Termination by Employer" |
| Family Code | 255 of 257 | Arts. 150 and 219 are absent from the source text |

Text repair:
- UTF-8 that had been read as Latin-1 is decoded back.
- Lead bytes whose continuation bytes were already lost upstream are handled as follows: before a contraction they become `'`; between letters they become ñ or Ñ; otherwise they are dropped.
- The E-Library header and footer are stripped.

### Decisions, into `phcases.db`

- **Scope:** all 31,180 Supreme Court decisions (1996 to 2024-04).
- **Chunking:** each decision is split fully into 480-token passages with one sentence of overlap, about 632k passages.
- **Search:** every passage has an e5 vector and is in FTS5. Search returns the best passage per decision.
- **Names:** parties appear as the Court wrote them; its own AAA/XXX pseudonyms are kept.
