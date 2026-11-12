# Phase 6: retire the AWS stack

Start only after the brother has used the desktop app for at least a week without a blocker, and
the owner has said to retire AWS. Every destructive step below needs its **own** owner yes. One
approval never carries over to the next step.

## Order

1. **Freeze the web app.** Remove the Amplify app `d29w361a9murqc`, or set it to a "moved to the
   Mac app" page. Ask the owner which.
2. **Export a safety copy.** Sync the S3 docs bucket and run a DynamoDB export to
   `~/Backups/kristoff-aws-final-<date>/`, then verify the counts. The owner chose to start fresh,
   so this is a backup only, not an import.
3. **Destroy the compute stack.** Run `terraform destroy` through the repo's `tf.sh` with the prod
   compute root. Before the destroy, print the backend state key and confirm it is `kristoff-prod`
   (rule 8: the backend picks the environment).
4. **Destroy the data stack.** It has `prevent_destroy` and deletion protection. Lifting them is
   its own commit and its own yes. Empty the versioned bucket (all versions) only after step 2 is
   verified.
5. **Bootstrap.** Its state is local in `backend/terraform/bootstrap/prod/`, with a backup in the
   tfstate bucket. Destroy the roles and SSM parameters first, then the tfstate bucket last.
6. **Account.** The owner chooses between closing `kristoff-prod` (815882254506) and keeping it
   empty. Remove the OU's SCPs only if the account is closed.
7. **Code cleanup PR** (`chore/retire-aws`):
   - Delete `backend/`, `web/services/api.js`, `web/scripts/mock-api.mjs` (if no longer used),
     `.github/workflows/deploy-*`, `docs/runbooks/owner-setup.md` and the web `config.json`
     handling.
   - The `api-desktop.js` export-parity check goes with `api.js`, so remove it in the same PR.
   - Update the README and `docs/contract.md`. The contract stays as the desktop IPC contract.

## Exit

- `aws resourcegroupstaggingapi get-resources --profile kristoff-prod` shows nothing billable, or
  the account is closed.
- The cleanup PR is merged by the owner. Memory and `docs/STATUS.md` are updated.
