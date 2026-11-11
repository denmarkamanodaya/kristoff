#!/usr/bin/env bash
# test-guards.sh — pins the behaviour of scripts/tf-plan-guard.jq and of tf.sh's refusals with inline fixtures. Offline:
# no AWS, no Terraform state. Every case states the exit code it expects; `--mutate` re-runs the delete/replace cases
# against a deliberately weakened guard and fails if one still blocks: a test that cannot fail is no test.
# Usage: bash backend/terraform/scripts/test-guards.sh [--mutate]
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TF="$HERE/.."
GUARD="$HERE/tf-plan-guard.jq"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP:?}"' EXIT
TODAY="2026-11-12"
fail=0

if [ "${1:-}" = "--mutate" ]; then
  # Weakened guard: the guarded-type delete rule is switched off. Every delete/replace case below must now pass the
  # guard; if one still blocks, it was blocked by something other than the rule it names.
  sed 's/((guarded_types | index(rc.type)) != null) and is_delete(rc)/false/' "$GUARD" >"$TMP/mutant.jq"
  GUARD="$TMP/mutant.jq"
  MUTATE=1
else
  MUTATE=0
fi

plan_json() { # plan_json <type> <address> <actions-json> [after-json]
  local after="${4-}"
  [ -n "$after" ] || after='{}'
  jq -n --arg t "$1" --arg a "$2" --argjson act "$3" --argjson after "$after" \
    '{format_version:"1.2", resource_changes:[{address:$a,type:$t,change:{actions:$act,after:$after}}]}'
}
allow_file() { # allow_file <address> <actions-json> <expires>
  jq -n --arg a "$1" --argjson act "$2" --arg e "$3" '{allow:[{address:$a,actions:$act,pr:1,expires:$e,reason:"test"}]}' >"$TMP/allow.json"
  echo "$TMP/allow.json"
}
no_allow() { echo '{"allow":[]}' >"$TMP/none.json"; echo "$TMP/none.json"; }

expect() { # expect <name> <want-exit> <stdin-file> <allow-file>
  local name="$1" want="$2" input="$3" allow="$4" got=0
  jq -n -r -f "$GUARD" --slurpfile allow "$allow" --arg today "$TODAY" <"$input" >"$TMP/out.txt" 2>&1 || got=$?
  if [ "$MUTATE" = 1 ] && [ "$want" = 1 ]; then want=0; fi # the weakened guard must let the red cases through
  if [ "$got" != "$want" ]; then
    echo "FAIL  $name: exit $got, wanted $want"
    sed 's/^/        /' "$TMP/out.txt"
    fail=1
  else
    echo "ok    $name"
  fi
}

N="$(no_allow)"
plan_json aws_dynamodb_table module.platform.module.table.aws_dynamodb_table.this '["no-op"]' >"$TMP/clean.json"
plan_json aws_dynamodb_table module.platform.module.table.aws_dynamodb_table.this '["delete"]' >"$TMP/table-delete.json"
plan_json aws_dynamodb_table module.platform.module.table.aws_dynamodb_table.this '["delete","create"]' >"$TMP/table-replace.json"
plan_json aws_s3_bucket module.platform.module.docs_bucket.aws_s3_bucket.this '["delete","create"]' >"$TMP/bucket-replace.json"
plan_json aws_ssm_parameter module.platform.aws_ssm_parameter.ai_enabled '["delete"]' >"$TMP/param-delete.json"
plan_json aws_amplify_app module.platform.module.web.aws_amplify_app.this '["delete","create"]' >"$TMP/amplify-replace.json"
plan_json aws_dynamodb_table module.platform.module.table.aws_dynamodb_table.this '["update"]' '{"point_in_time_recovery":[{"enabled":false}]}' >"$TMP/pitr-off.json"
plan_json aws_dynamodb_table module.platform.module.table.aws_dynamodb_table.this '["update"]' '{"deletion_protection_enabled":false}' >"$TMP/dp-off.json"
plan_json aws_s3_bucket_public_access_block module.platform.module.docs_bucket.aws_s3_bucket_public_access_block.this '["update"]' '{"block_public_acls":true,"block_public_policy":false,"ignore_public_acls":true,"restrict_public_buckets":true}' >"$TMP/bpa.json"
plan_json aws_s3_bucket_versioning module.platform.module.docs_bucket.aws_s3_bucket_versioning.this '["update"]' '{"versioning_configuration":[{"status":"Suspended"}]}' >"$TMP/versioning.json"
plan_json aws_lambda_function module.compute.module.fn.aws_lambda_function.this '["delete","create"]' >"$TMP/lambda-replace.json" # not a guarded type
echo -n "" >"$TMP/empty.json"
echo '{"not":"a plan"}' >"$TMP/not-a-plan.json"

expect "clean plan passes" 0 "$TMP/clean.json" "$N"
expect "replacing a function (unguarded type) passes" 0 "$TMP/lambda-replace.json" "$N"
expect "table delete is blocked" 1 "$TMP/table-delete.json" "$N"
expect "table replace is blocked" 1 "$TMP/table-replace.json" "$N"
expect "docs bucket replace is blocked" 1 "$TMP/bucket-replace.json" "$N"
expect "kill-switch parameter delete is blocked" 1 "$TMP/param-delete.json" "$N"
expect "Amplify app replace is blocked" 1 "$TMP/amplify-replace.json" "$N"
if [ "$MUTATE" = 0 ]; then
  # Rules that are not driven by guarded_types stay red even under the mutant, so they are not part of --mutate.
  expect "PITR off is blocked" 1 "$TMP/pitr-off.json" "$N"
  expect "table deletion protection off is blocked" 1 "$TMP/dp-off.json" "$N"
  expect "Block Public Access weakened is blocked" 1 "$TMP/bpa.json" "$N"
  expect "versioning suspended is blocked" 1 "$TMP/versioning.json" "$N"
  expect "empty input is blocked" 1 "$TMP/empty.json" "$N"
  expect "input that is not a plan is blocked" 1 "$TMP/not-a-plan.json" "$N"
  expect "an exact, unexpired allow entry lets a delete through" 0 "$TMP/table-delete.json" "$(allow_file module.platform.module.table.aws_dynamodb_table.this '["delete"]' 2026-12-31)"
  expect "an allow entry for other actions does not" 1 "$TMP/table-delete.json" "$(allow_file module.platform.module.table.aws_dynamodb_table.this '["delete","create"]' 2026-12-31)"
  expect "an allow entry for another address does not" 1 "$TMP/table-delete.json" "$(allow_file module.platform.module.other.aws_dynamodb_table.this '["delete"]' 2026-12-31)"
  expect "an expired allow entry does not" 1 "$TMP/table-delete.json" "$(allow_file module.platform.module.table.aws_dynamodb_table.this '["delete"]' 2026-10-18)"

  # tf.sh refusals (exit 65) and usage (64) happen before any AWS or Terraform call.
  tfsh() { bash "$TF/tf.sh" "$@" >"$TMP/tfsh.txt" 2>&1 && echo 0 || echo $?; }
  check_tfsh() { # check_tfsh <name> <want> <args...>
    local name="$1" want="$2"
    shift 2
    local got
    got="$(tfsh "$@")"
    if [ "$got" = "$want" ]; then echo "ok    $name"; else echo "FAIL  $name: exit $got, wanted $want"; sed 's/^/        /' "$TMP/tfsh.txt"; fail=1; fi
  }
  check_tfsh "tf.sh: no arguments is a usage error" 64
  check_tfsh "tf.sh: unknown stage is a usage error" 64 dev platform init
  check_tfsh "tf.sh: unknown stack is a usage error" 64 prod network init
  # The account-id case runs on a private copy whose tfvars carry a placeholder: the real env file holds the real
  # account id, and running the real tree here would reach `aws sts` and `terraform init` against prod.
  fake="$TMP/tfroot"
  mkdir -p "$fake/roots/platform" "$fake/env"
  cp "$TF/tf.sh" "$fake/tf.sh"
  : >"$fake/roots/platform/main.tf"
  printf 'account_id = "TODO_PROD_ACCOUNT_ID"\n' >"$fake/env/prod.tfvars"
  : >"$fake/env/prod.platform.tfvars"
  : >"$fake/env/prod.platform.backend.hcl"
  got="$(bash "$fake/tf.sh" prod platform init >"$TMP/tfsh.txt" 2>&1 && echo 0 || echo $?)"
  if [ "$got" = 65 ] && grep -q "no 12-digit account_id" "$TMP/tfsh.txt"; then echo "ok    tf.sh: a TODO account id is refused"; else echo "FAIL  tf.sh: TODO account id exit $got"; sed 's/^/        /' "$TMP/tfsh.txt"; fail=1; fi
  (
    export TF_VAR_stage=prod
    got="$(tfsh prod platform init)"
    [ "$got" = 65 ] && echo "ok    tf.sh: TF_VAR_* is refused" || { echo "FAIL  tf.sh: TF_VAR_* exit $got, wanted 65"; exit 1; }
  ) || fail=1
  (
    export TF_CLI_ARGS_plan=-lock=false
    got="$(tfsh prod platform plan)"
    [ "$got" = 65 ] && echo "ok    tf.sh: TF_CLI_ARGS* is refused" || { echo "FAIL  tf.sh: TF_CLI_ARGS exit $got, wanted 65"; exit 1; }
  ) || fail=1
  check_tfsh "tf.sh: apply without a saved plan is refused" 65 prod platform apply
  # The SecureStrings are owner-set: no SecureString parameter and no secret value resource may exist anywhere in the tree
  # (the provider reads a parameter back decrypted, so it would land in the state).
  if grep -rlqE --include='*.tf' 'type[[:space:]]*=[[:space:]]*"SecureString"|resource "aws_secretsmanager_secret_version"' "$TF"; then
    echo "FAIL  no SecureString parameter or secret value in any .tf"; grep -rlnE --include='*.tf' 'type[[:space:]]*=[[:space:]]*"SecureString"|resource "aws_secretsmanager_secret_version"' "$TF" | sed 's/^/        /'; fail=1
  else
    echo "ok    no SecureString parameter or secret value in any .tf (secrets stay owner-set)"
  fi
fi

if [ "$fail" = 0 ]; then echo "all guard tests passed"; else echo "GUARD TESTS FAILED"; exit 1; fi
