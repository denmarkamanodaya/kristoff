# Phase 3: PH law index

Goal: a read-only `phlaws.db` built from the Hugging Face dataset, plus a `LawIndex` adapter that
answers hybrid searches in under 150 ms on an M1.

## 1. Ingest tool (`crates/kristoff-ingest`, a bin, never shipped in the app)

`cargo run -p kristoff-ingest --release -- --source desktop/data/source --out desktop/data/phlaws.db`

1. **Fetch.** `scripts/fetch-dataset.sh` downloads `laws.parquet` and `articles.parquet` from
   `endomorphosis/ipfs_philippines_laws` at a pinned revision. The revision hash and SHA-256 sums
   go in `desktop/data/dataset.lock`. `desktop/data/` is gitignored except `dataset.lock`.
2. **Read** with the `parquet` crate. Columns: law id, identifier, title, date, `law_status`,
   `source_url`; article law id, label, order, text.
   - Print the real schema on the first run and fix the column names in one `columns.rs` constant
     set. Never guess them.
3. **Clean.**
   - Trim stray leading quotes and characters, collapse whitespace, NFC-normalise.
   - Mark rows that end mid-sentence or at a known cell cap as `truncated=1`.
   - Drop empty articles. Print the count of each fix.
4. **Chunk.**
   - One chunk per article, prefixed with
     `"<identifier> — <title> · <article label>\n"`.
   - Split articles over about 1,500 tokens on paragraph breaks and repeat the prefix. Use the e5
     tokenizer to count tokens.
5. **Index.**
   - Tables: `laws`, `chunks(id, law_id, label, ord, text, truncated)`.
   - `chunks_fts`: FTS5, external content, `tokenize='porter unicode61 remove_diacritics 2'`, over
     prefix + text.
   - `vectors(chunk_id, v BLOB)`: 384 × int8, per-vector scale stored as f32.
   - `meta(key, value)`: dataset revision, model revision, build date, counts.
6. **Embed** with the same `CandleEmbedder` the app uses, prefix `"passage: "`, batches of 64 on
   Metal. Resumable: skip `chunk_id`s that already have vectors.
7. **Finish:** `VACUUM`, `PRAGMA journal_mode=DELETE`, then print the size and the counts.

The model files come from `scripts/fetch-model.sh`: e5-small safetensors converted to f16, plus
`tokenizer.json`. The revision and SHA-256 sums are pinned in `desktop/data/model.lock`.

## 2. `LawIndex` adapter (in the app)

- Open `phlaws.db` read-only from the bundle resources (`immutable=1`). Load all vectors into
  memory at first use: about 36 MB, a one-time cost of under 1 s.
- `search(query, filters, limit = 8)`:
  1. Run in parallel: FTS5 BM25 (30 candidates; quoted terms, no raw MATCH syntax from user text)
     and kNN (embed `"query: " + text`, int8 dot product, 30 candidates).
  2. Reference parser: a pure core function that reads `RA 9262`, `R.A. No. 9262`, `Republic Act
     9262`, `Sec. 5`, `Section 5`, `Art. III` and `Article 3`, and boosts exact hits.
  3. Merge by reciprocal rank fusion (K = 60) and apply the year and law-number filters.
- `get_article(law_number, article)` returns that chunk plus its neighbours `ord ± 1`.
- Results are domain types (`LawHit {law, label, text, source_url, status, truncated}`). No rows
  cross the port.

## 3. Smoke test

- `desktop/crates/kristoff-adapters/tests/law_smoke.rs` runs 20 look-ups from
  `contracts/fixtures/law-smoke.json`, for example "Article III Sec. 2 Constitution", "RA 10173
  data privacy", "RA 9262 Sec. 5" and "Labor Code just causes".
- Each expected answer is a law identifier plus an article label. The test is ignored by default
  (it needs the built db) and run in CI-local when `desktop/data/phlaws.db` exists.

## Exit

- `phlaws.db` is built. Its size and counts are recorded in the PR (expected ~150–200 MB, about
  93k chunks).
- The smoke test passes: 20/20 in the top 5.
- Five Taglish meaning queries return the relevant article in the top 8, recorded in the PR.
  Example: "pwede bang tanggalin sa trabaho nang walang notice".
- Search p95 is under 150 ms over 100 runs, recorded in the PR.
- The reference parser has property tests.
