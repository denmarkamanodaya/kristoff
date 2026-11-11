#!/usr/bin/env bash
# tf.sh — the ONLY way to run Terraform against a kristoff stage (owner ruling R8, stack/terraform.md). It always pairs,
# for one <stage>:
#   backend  env/<stage>.<stack>.backend.hcl
#   inputs   env/<stage>.tfvars + env/<stage>.<stack>.tfvars
#   data dir roots/<stack>/.terraform-<stage>   (one per stage: switching stage never re-initialises, never migrates)
# so a stage cannot be mixed from the command line. The stage guard in the root (modules/stage-guard/v1) re-checks the
# result inside Terraform: caller account == account_id, initialised backend == kristoff-tfstate-<account_id> /
# kristoff/<stage>/<stack>.tfstate. CI and humans use only this script.
#
# Usage: backend/terraform/tf.sh prod <platform|compute> <command> [option]   (Kristoff has one stage: prod)
#   init                 terraform init with the paired backend (never -reconfigure, never -migrate-state)
#   validate             offline: init -backend=false into .terraform-validate, then validate (no AWS, no tfvars)
#   plan [--read-only]   init, plan -detailed-exitcode -out=<stage>.tfplan, then the plan guard on the saved plan;
#                        --read-only adds -lock=false (the PR plan role cannot write the lock object)
#   apply                apply the saved <stage>.tfplan from `plan` (nothing else), then delete it
#   output <name>        terraform output -raw <name>
#   output-json          terraform output -json (every output as one JSON object; lists need it)
# Exit: init/validate/apply/output/output-json 0 = ok; plan 0 = no changes, 2 = changes (guard passed), 1 = error or BLOCKED;
# 64 = usage error, 65 = refused (unsafe environment, wrong account, missing file).
# Requires: terraform, jq; the aws CLI for every command but validate (account preflight). Run from anywhere.
# The agent never applies: `apply` is for CI and the owner (stack/terraform.md).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
usage() { echo "usage: tf.sh prod <platform|compute> <init|validate|plan [--read-only]|apply|output <name>|output-json>" >&2; exit 64; }
refuse() { echo "tf.sh: REFUSED — $*" >&2; exit 65; }

[ "$#" -ge 3 ] || usage
STAGE="$1" STACK="$2" CMD="$3"
shift 3
case "$STAGE" in prod) ;; *) usage ;; esac
case "$STACK" in platform | compute) ;; *) usage ;; esac

ROOT="$HERE/roots/$STACK"
BACKEND="$HERE/env/$STAGE.$STACK.backend.hcl"
VARS_STAGE="$HERE/env/$STAGE.tfvars"
VARS_STACK="$HERE/env/$STAGE.$STACK.tfvars"
PLAN_FILE="$STAGE.tfplan" # relative to $ROOT; *.tfplan is git-ignored
for f in "$ROOT/main.tf" "$BACKEND" "$VARS_STAGE" "$VARS_STACK"; do [ -f "$f" ] || refuse "missing $f"; done

# Nothing outside the paired files may feed the root: TF_VAR_* and TF_CLI_ARGS* would inject values or flags, and
# Terraform auto-loads terraform.tfvars / *.auto.tfvars from the root directory.
injected="$(env | grep -oE '^(TF_VAR_|TF_CLI_ARGS)[A-Za-z0-9_]*' || true)"
if [ -n "$injected" ]; then
  refuse "unset every TF_VAR_* and TF_CLI_ARGS* variable: $(printf '%s\n' "$injected" | tr '\n' ' ')"
fi
autoloaded=()
for f in "$ROOT"/terraform.tfvars "$ROOT"/terraform.tfvars.json "$ROOT"/*.auto.tfvars "$ROOT"/*.auto.tfvars.json; do
  [ -e "$f" ] && autoloaded+=("$f")
done
[ "${#autoloaded[@]}" -eq 0 ] || refuse "auto-loaded variable files in $ROOT: ${autoloaded[*]}"

# The account the tfvars claim, compared with the credentials before Terraform touches the backend (the stage guard
# repeats the check inside the plan; this one fails earlier and also protects `init` and `output`).
preflight_account() {
  local want have
  want="$(sed -nE 's/^account_id[[:space:]]*=[[:space:]]*"([0-9]{12})".*/\1/p' "$VARS_STAGE")"
  [ -n "$want" ] || refuse "$VARS_STAGE has no 12-digit account_id (docs/runbooks/owner-setup.md)"
  have="$(aws sts get-caller-identity --query Account --output text)" || refuse "aws sts get-caller-identity failed (log in: aws sso login --sso-session dk; export AWS_PROFILE=kristoff-prod)"
  [ "$have" = "$want" ] || refuse "env/$STAGE.tfvars is account $want, the credentials are account $have (check AWS_PROFILE)"
}

tf() { (cd "$ROOT" && TF_DATA_DIR=".terraform-$STAGE" TF_IN_AUTOMATION=1 terraform "$@"); }
var_files=(-var-file="$VARS_STAGE" -var-file="$VARS_STACK")

do_init() {
  preflight_account
  tf init -input=false -backend-config="$BACKEND"
}

case "$CMD" in
  init)
    [ "$#" -eq 0 ] || usage
    do_init
    ;;
  validate)
    [ "$#" -eq 0 ] || usage
    (cd "$ROOT" && TF_DATA_DIR=.terraform-validate terraform init -backend=false -input=false >/dev/null && TF_DATA_DIR=.terraform-validate terraform validate)
    ;;
  plan)
    lock=()
    [ "$#" -le 1 ] || usage
    case "${1:-}" in "") ;; --read-only) lock=(-lock=false) ;; *) usage ;; esac
    do_init
    rm -f "${ROOT:?}/${PLAN_FILE:?}"
    set +e
    tf plan -input=false -detailed-exitcode ${lock[@]+"${lock[@]}"} "${var_files[@]}" -out="$PLAN_FILE"
    rc=$?
    set -e
    [ "$rc" -eq 0 ] || [ "$rc" -eq 2 ] || { echo "tf.sh: $STAGE/$STACK plan failed (exit $rc)" >&2; exit 1; }
    # The guard reads the SAVED plan; -n makes an empty or failed `show` a failure, never a pass.
    tf show -json "$PLAN_FILE" | jq -n -r -f "$HERE/scripts/tf-plan-guard.jq" \
      --slurpfile allow "$HERE/plan-guard-allow.json" --arg today "$(date -u +%F)" || exit 1
    exit "$rc"
    ;;
  apply)
    [ "$#" -eq 0 ] || usage
    [ -f "$ROOT/$PLAN_FILE" ] || refuse "no saved plan $ROOT/$PLAN_FILE — run tf.sh $STAGE $STACK plan first (only a guarded plan is applied)"
    preflight_account
    tf apply -input=false "$PLAN_FILE"
    rm -f "${ROOT:?}/${PLAN_FILE:?}"
    ;;
  output)
    [ "$#" -eq 1 ] || usage
    preflight_account
    tf output -raw "$1"
    ;;
  output-json)
    [ "$#" -eq 0 ] || usage
    preflight_account
    tf output -json
    ;;
  *) usage ;;
esac
