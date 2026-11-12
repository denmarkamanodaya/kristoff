# Phase 5: evaluation and the DMG

Goal: measured answer quality, then a DMG the owner can hand to his brother.

## 1. Evaluation

- `contracts/fixtures/eval/questions.json` holds 40 questions:
  - 20 statute look-ups, each with the expected identifier and label.
  - 10 concept questions (e.g. "grounds for annulment", "what is a valid warrantless arrest").
  - 10 mixed questions that need a sample decision PDF. Use public SC decisions only, stored under
    `desktop/eval/pdfs/` (gitignored, fetched by script).
- `cargo run -p kristoff-ingest --bin eval` runs every question through the real ask use case
  against Sonnet 5.5. It writes `desktop/eval/report-<date>.json` with retrieval hits, citations,
  tokens and cost.
- Check every statutory sentence: one that names an RA, article or section must have a law
  citation. Use a regex for law references plus the citations list.
- Pass when:
  - At least 90% of look-ups have the right article in the top 8.
  - Zero uncited statutory claims.
  - Average cost per statute question is at most US$0.05.
- Cost: about US$2–3 per eval run. Run it at most twice in this phase.

## 2. Packaging (`desktop/scripts/build-dmg.sh`)

1. Check that `KRISTOFF_ANTHROPIC_KEY` and `KRISTOFF_TOTP_SECRET` are set.
   - The owner creates a dedicated Console key with a monthly spend limit (see `../plan.md`
     security note).
   - The TOTP secret is read from the existing SecureString with `aws ssm get-parameter
     --with-decryption --profile kristoff-prod`. It is never printed.
2. Check `phlaws.db` and the model files against `dataset.lock` and `model.lock`.
3. Build: `npm --prefix web run build -- --target desktop`, then
   `cargo tauri build --target aarch64-apple-darwin --bundles app,dmg`. The resources are
   `phlaws.db`, `models/e5-small/*` and `THIRD_PARTY_NOTICES`.
4. Run `codesign --verify --deep` on the ad-hoc app. Print the size, and copy the DMG to
   `desktop/dist/Kristoff-<version>-arm64.dmg` (gitignored).
5. Write SHA-256 sums beside the DMG.

Generate `THIRD_PARTY_NOTICES` from `cargo about` or a hand list. Include the dataset's licence and
the e5 model's MIT licence. If `cargo about` would be a new tool install, ask first; otherwise
write the hand list.

## 3. Docs

- `docs/desktop/install-for-brother.md`:
  - How the DMG reaches him (AirDrop, or a USB stick), drag to Applications, and the exact
    first-open steps recorded in S1.
  - The authenticator code.
  - Where his data lives, and how to back it up (copy
    `~/Library/Application Support/app.kristoff.desktop/`).
  - What "file deleted" means.
  - Owner test checklist (owner runs it on the final DMG):
    1. Install it on a second account.
    2. Unlock it.
    3. Upload a PDF.
    4. Ask a mixed question.
    5. Open both kinds of citation.
    6. Dictate.
    7. Delete the file.
    8. Quit and reopen; the gate asks for a code again.
- `README.md`: a new "Desktop app" section. Mark the web and AWS sections as "until retirement".

## Exit

- The eval report passes and is linked in the PR.
- The DMG is built, about 450 MB. Claude installs it on a second macOS account and runs the owner
  checklist first; the results go in the PR.
- The owner runs the checklist on the final DMG and says yes. Then hand the DMG to his brother.
