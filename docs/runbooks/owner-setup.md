# Kristoff owner setup: bootstrap, secrets, GitHub, first deploy

Every step here is one only the owner can do (bootstrap apply, secret values, GitHub settings). Do the steps in order:
the secrets (step 4) must exist before the first deploy (step 7), because the functions read them at cold start and
fail closed without them. Each step gives the command, what you should see, and how to undo it.

Conventions: run commands from the repo root; the AWS profile is `kristoff-prod` (SSO session `dk`); the Region is
always `ap-southeast-2`. Terraform layout and rules: `backend/terraform/README.md`. Names and SSM parameters:
`docs/contract.md`.

## Already done (2026-11-12, by the lead; recorded, do not repeat)

| Step | Result |
|---|---|
| OU `kristoff` | `ou-2con-ekrgjz2b` under root `r-2con` |
| Account `kristoff-prod` | `815882254506`, in the OU, IAM access to billing allowed |
| Permission set `KristoffAdmin` | `ps-8210cc9bcc638d0e` (AdministratorAccess), assigned to the owner |
| SCPs on the OU | `kristoff-guardrails` `p-cqyosgio`, `kristoff-region-allowlist` `p-73ijv8ub`, `kristoff-cost-guardrail` `p-qla3ur6x`; Access Analyzer 0 findings; probe: `iam:CreateUser` and `us-east-1` denied |
| CLI profile | `kristoff-prod` in `~/.aws/config`, `sso_session = dk` |
| GitHub repo | `denmarkamanodaya/kristoff`, id `1454246629` (owner id `5304296`) |

Details and the attach record: `backend/terraform/bootstrap/scp/README.md`.

## The steps

### 1. Log in and check the account

```bash
aws sso login --sso-session dk
export AWS_PROFILE=kristoff-prod
aws sts get-caller-identity --query Account --output text
```
You should see `815882254506`. If not, check `AWS_PROFILE`.
Undo: nothing to undo.

### 2. Check the literals

The alert mailbox is `denmarkamanodaya@gmail.com` in `backend/terraform/bootstrap/prod/main.tf` (Budget mails) and in
`backend/terraform/env/prod.platform.tfvars` (alarm mails). No `TODO_` placeholder is left:
```bash
grep -rn "TODO_" backend/terraform --include='*.tf' --include='*.tfvars' | grep -v '/tests/' | grep -v 'TODO_\*\|TODO_\[A-Z'
```
You should see no line.
Undo: nothing to undo.

### 3. Bootstrap apply (state bucket, OIDC deploy role, Lambda boundary, $20 Budget, CloudTrail, GuardDuty)

Read the plan before you apply it. Local state on purpose: it stays in `bootstrap/prod/`, git-ignored.
```bash
cd backend/terraform/bootstrap/prod
terraform init
terraform plan -out=bootstrap.tfplan
terraform apply bootstrap.tfplan
terraform output
cd -
```
You should see the plan create (among others) the state bucket `kristoff-tfstate-815882254506`, trail bucket,
OIDC provider, roles `kristoff-prod-ci-deploy` and `kristoff-prod-ci-plan`, policy `kristoff-prod-lambda-boundary`,
budget `kristoff-prod-monthly`, GuardDuty detector), and no change to anything else. `terraform output` prints
`tfstate_bucket` (must equal the bucket in `backend/terraform/env/prod.*.backend.hcl`) and `ci_deploy_role_arn`
(needed in step 6). Back up `bootstrap/prod/terraform.tfstate` outside git (password manager attachment).
Undo: the state, trail and table buckets carry `prevent_destroy`; removing the bootstrap is a deliberate manual
teardown (empty the buckets, drop `prevent_destroy`, `terraform destroy`). Do not do it casually.

### 4. Create the three secrets in SSM (before the first deploy)

Terraform never holds these values; it only passes their names to the functions. Use `--type SecureString` (default
key `aws/ssm`). Nothing here is echoed or written to shell history.

**4a. Anthropic API key.** Create a new key for prod in the Anthropic Console and set a monthly spend limit there.
```bash
read -rs ANTHROPIC_KEY && aws ssm put-parameter --name /kristoff/prod/anthropic-api-key --type SecureString \
  --value "$ANTHROPIC_KEY" --overwrite && unset ANTHROPIC_KEY
```

**4b. Session HMAC key** (at least 32 random bytes, base64):
```bash
node backend/scripts/gen-session-key.mjs    # see its header for the exact output and options
```

**4c. TOTP secret** (RFC 6238, base32). The script prints the `otpauth://` URI / QR code your brother scans into
Google Authenticator:
```bash
node backend/scripts/enroll-totp.mjs        # see its header for the exact output and options
```
If a script prints the value instead of writing it, store it the same way as 4a (`read -rs`, then
`aws ssm put-parameter --name /kristoff/prod/<session-hmac-key|totp-secret> --type SecureString --overwrite`).

Check (names and types only, never values):
```bash
aws ssm describe-parameters --parameter-filters Key=Name,Option=BeginsWith,Values=/kristoff/prod/ \
  --query 'Parameters[].[Name,Type]' --output table
```
You should see the three names with type `SecureString` (`ai-enabled` appears only after step 7).
Undo: `aws ssm delete-parameter --name <name>`; rotate a value by repeating its command (`--overwrite`). A new
TOTP secret means re-enrolling the authenticator; a new session key logs out every open session.

### 5. GitHub environment `prod`

```bash
gh api -X PUT repos/denmarkamanodaya/kristoff/environments/prod \
  -F 'deployment_branch_policy[protected_branches]=false' -F 'deployment_branch_policy[custom_branch_policies]=true'
gh api -X POST repos/denmarkamanodaya/kristoff/environments/prod/deployment-branch-policies -f name=main -f type=branch
```
You should see JSON for the environment, then for the branch policy `main`. Deploys can now run only from `main`.
Undo: `gh api -X DELETE repos/denmarkamanodaya/kristoff/environments/prod`.

### 6. GitHub variable `AWS_DEPLOY_ROLE_ARN` on the `prod` environment

```bash
ROLE_ARN=$(cd backend/terraform/bootstrap/prod && terraform output -raw ci_deploy_role_arn)
gh variable set AWS_DEPLOY_ROLE_ARN --env prod --repo denmarkamanodaya/kristoff --body "$ROLE_ARN"
gh variable list --env prod --repo denmarkamanodaya/kristoff
```
You should see `AWS_DEPLOY_ROLE_ARN` = `arn:aws:iam::815882254506:role/kristoff/ci/kristoff-prod-ci-deploy`. It is a
variable, not a secret: the role trusts only the OIDC subject
`repo:denmarkamanodaya@5304296/kristoff@1454246629:environment:prod`.
Undo: `gh variable delete AWS_DEPLOY_ROLE_ARN --env prod --repo denmarkamanodaya/kristoff`.

### 7. First deploy

Merge the rebuild into `main` (PR `develop` -> `main`), then:
```bash
gh workflow run deploy-prod.yml --ref main --repo denmarkamanodaya/kristoff
gh run watch --repo denmarkamanodaya/kristoff
```
You should see the run build the bundles, plan + guard + apply `platform`, then `compute`, write `config.json`, and
finish with `amplify job <id>: SUCCEED`. Confirm the SNS subscription mail that arrives at the alert address.
The compute plan reads the web origin from the platform state, so platform always runs first (the workflow does).
If the run stops on `AccessDenied`, the deploy-role policy misses an action: note the action from the log, add it in
`backend/terraform/bootstrap/account/ci_deploy_policy_*.tf` (PR), re-run step 3, re-run this step.
Undo: re-run with the previous commit (the functions' `live` alias then points at the previous code). The table and
the bucket are never destroyed by a deploy (plan guard + deny policy).

### 8. Open the site

```bash
bash backend/terraform/tf.sh prod platform output web_origin
```
You should see `https://main.<app id>.amplifyapp.com`: the site URL (owner ruling 2026-11-12: no custom domain). It is
also the only origin the API, the stream URL and the docs bucket accept (CORS); the compute stack reads it from the
platform state, so nothing has to be copied by hand. Send this URL to your brother.
Undo: nothing to undo.

### 9. Smoke check

- Open the URL from step 8: a wrong code gives an error; five wrong codes lock for 15 minutes; the
  right code from Google Authenticator opens the library.
- Upload a PDF, ask a question, see a streamed answer with sources; reload the page and the TOTP prompt returns.
- `curl -i "$(bash backend/terraform/tf.sh prod compute output api_base_url)/v1/docs"` answers `401`.

### Emergency stop (any time)

```bash
aws ssm put-parameter --name /kristoff/prod/ai-enabled --type String --value false --overwrite
```
Every question then answers `AI_DISABLED` within a minute (60 s cache). Deploys never revert it. Undo: `--value true`.
