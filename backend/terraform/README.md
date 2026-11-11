# Kristoff Terraform — one root per stack, the stage comes from env/ through tf.sh

AWS Sydney (`ap-southeast-2`), one account, one stage: `kristoff-prod` (`815882254506`, OU `kristoff`). Conventions:
the `dk-coding` skill and `~/Projects/.claude/rules/stack/terraform.md` (stage selection by `tf.sh` + `env/` files).
Names, routes, timeouts and SSM names: `docs/contract.md`. The owner's step-by-step: `docs/runbooks/owner-setup.md`.
Adapted from `~/Projects/dexfs/backend/terraform` (own copy; dev stage, Cognito, queues and vectors dropped).

```
backend/terraform/
  bootstrap/
    scp/                     three SCP JSON files (attached 2026-11-12) + README with their ids
    account/                 bootstrap module: state bucket, account Block Public Access, password policy, CloudTrail,
                             $20 Budget, GitHub OIDC provider, ci-plan + ci-deploy roles, Lambda permission boundary,
                             GuardDuty
    prod/main.tf             OWNER-RUN root with LOCAL state: literal stage, account id, repo id, alert e-mail
  modules/<name>/v1/         versioned adapters: stage-names, stage-guard, lambda-fn, lambda-stream-url, http-api,
                             dynamodb-table, s3-bucket, amplify-app — main/variables/outputs/versions, README
                             (terraform-docs block), tests/v1.tftest.hcl
  stacks/platform/           data + web: table, docs bucket, ai-enabled kill switch, alerts topic, Amplify app
                             (served only on https://main.<default domain>; output web_origin)
  stacks/compute/            13 HTTP functions + authorizer, HTTP API, ask-stream function URL, alarms; IAM and env per
                             function derived from backend/lambda-scope.json (scope.tf guards it)
  roots/<stack>/             THE root of each stack: partial S3 backend, provider, stage guard, the stack call;
                             roots/compute reads web_origin from the platform state (terraform_remote_state)
  env/prod.tfvars            stage inputs shared by both roots (stage, account_id, region)
  env/prod.<stack>.tfvars    stack inputs; non-secret
  env/prod.<stack>.backend.hcl  bucket kristoff-tfstate-<account_id>, key kristoff/prod/<stack>.tfstate, lockfile
  tf.sh                      the ONLY Terraform entry point: pairs the env files of the stage
  scripts/tf-plan-guard.jq   blocks delete/replace of the table, buckets, log groups, OIDC provider, KMS keys, the
                             kill-switch parameter and the Amplify app, and PITR / deletion-protection /
                             versioning / Block Public Access flips
  scripts/test-guards.sh     pins the guard and tf.sh refusals (and `--mutate` proves the tests can fail)
  plan-guard-allow.json      time-boxed exemptions for the guard (same PR as the destructive change)
  .tflint.hcl                tflint + aws ruleset configuration (CI only; not installed locally)
```

## Rules

1. **Run Terraform only through `tf.sh`** — `bash backend/terraform/tf.sh prod <platform|compute> <cmd>` (`init`,
   `validate`, `plan [--read-only]`, `apply`, `output <name>`, `output-json`). It refuses `TF_VAR_*`, `TF_CLI_ARGS*`,
   auto-loaded tfvars and credentials of another account (exit 65). Offline checks (`init -backend=false`, `validate`,
   `test`) may run directly. The owner-run `bootstrap/prod` root is run from its directory.
2. **The stage guard re-checks inside Terraform** (`modules/stage-guard/v1`): account id equals the caller, the
   initialised backend is `kristoff-tfstate-<account_id>` / `kristoff/prod/<stack>.tfstate`, and the state's own
   recorded identity matches.
3. **Saved plan + guard + apply the same file.** `tf.sh plan` writes `roots/<stack>/prod.tfplan`, pipes
   `terraform show -json` through the guard and exits 0 (no changes), 2 (changes, guard passed) or 1. `tf.sh apply`
   applies only that saved plan. CI applies on a manual `deploy-prod` run from `main`; an agent never applies.
4. **Secrets never enter Terraform.** The three SecureStrings (`/kristoff/prod/anthropic-api-key`, `totp-secret`,
   `session-hmac-key`) are created by the owner (runbook); Terraform only passes their names. The AWS provider reads a
   parameter back decrypted on every refresh, so a Terraform-managed secret would land in the state. Only the String
   kill switch `/kristoff/prod/ai-enabled` is a Terraform resource (value ignored after create).
5. **Names come from one place**: `modules/stage-names/v1`, also the Lambda data-action ceiling
   (`lambda_dynamodb_actions`, `lambda_s3_actions`) shared by the boundary and the compute scope check.
6. **Lambda bundles and scope**: the compute stack reads `backend/dist/<function key>.zip` and
   `backend/lambda-scope.json` (shape: `stacks/compute/tests/fixtures/lambda-scope.json`). A key without a bundle
   deploys `stacks/compute/placeholder/stub.zip` (503 `NOT_DEPLOYED`); a missing scope file fails the plan.

## Gates (offline)

```bash
terraform fmt -check -recursive backend/terraform
shellcheck backend/terraform/tf.sh backend/terraform/scripts/*.sh
bash backend/terraform/scripts/test-guards.sh && bash backend/terraform/scripts/test-guards.sh --mutate
for s in platform compute; do bash backend/terraform/tf.sh prod $s validate; done
( cd backend/terraform/bootstrap/prod && terraform init -backend=false && terraform validate )
for m in backend/terraform/modules/*/v1 backend/terraform/stacks/* backend/terraform/bootstrap/account; do ( cd "$m" && terraform init -backend=false && terraform test ); done
trivy config --severity HIGH,CRITICAL backend/terraform
for m in backend/terraform/modules/*/v1; do terraform-docs markdown table --output-file README.md --output-mode inject --output-check "$m"; done
cd backend/terraform && tflint --init && tflint --recursive --config "$(pwd)/.tflint.hcl"   # CI only so far
```

## Bootstrap state

`bootstrap/prod` uses LOCAL state, applied once by the owner. The state holds ARNs and names only and is git-ignored.
Back it up outside git (a private folder or a password-manager attachment) after each apply: losing it means
re-importing the account resources.

## Versioning a module

Behaviour change after the first prod apply = copy `modules/<name>/v1/` to `v2/`, change it there, update the README
head (`Supersedes: v1 · Change · Rollback`), add `tests/v2.tftest.hcl`, switch the caller's `source`, add `moved {}`
blocks for any address change, read the plan in full. Before the first prod apply, edit `v1` in place.
