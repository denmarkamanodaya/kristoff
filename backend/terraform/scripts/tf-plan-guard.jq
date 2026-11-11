# tf-plan-guard.jq — blocks destructive or weakening changes in a saved Terraform plan (stack/terraform.md).
#
# Usage (always under `set -o pipefail`, always with -n so an EMPTY stdin is an error, never a silent pass):
#   terraform show -json <stage>.tfplan | jq -n -r -f scripts/tf-plan-guard.jq \
#     --slurpfile allow plan-guard-allow.json --arg today "$(date -u +%F)"
# Output: resource ADDRESSES and reasons only — plan JSON may contain values, and none is ever printed.
# Exit: 1 on any blocked change (halt_error), 0 when clean. An allowlist entry exempts one address only when
# address AND the exact action set match AND `expires` (YYYY-MM-DD, UTC) is today or later.

# The data-bearing or hard-to-recreate types: the table, the docs bucket (and its versioning), log groups, the OIDC
# provider, KMS keys, the kill-switch parameter (a recreate would reset an owner-set value) and the Amplify app (a
# recreate changes its default domain, which is the web origin and the site URL).
def guarded_types: [
  "aws_dynamodb_table", "aws_s3_bucket", "aws_s3_bucket_versioning", "aws_cloudwatch_log_group",
  "aws_iam_openid_connect_provider", "aws_kms_key", "aws_ssm_parameter", "aws_amplify_app"
];

def fail(msg): (msg + "\n") | halt_error(1);

def plan: (try input catch null);

def allow_entries: (($allow // [])[0].allow // []);

def matching_allow(rc):
  [ allow_entries[]
    | select(.address == rc.address)
    | select(((.actions // []) | sort) == ((rc.change.actions // []) | sort))
    | select((.expires // "0000-00-00") >= $today)
  ] | first // null;

def actions(rc): (rc.change.actions // []);
def is_delete(rc): (actions(rc) | index("delete")) != null;
def survives(rc): (actions(rc) | index("create")) != null or (actions(rc) == ["update"]) or (actions(rc) == ["no-op"]) or (actions(rc) == ["read"]);
def after(rc): (rc.change.after // {});

def pitr_off(rc): ((after(rc).point_in_time_recovery // []) | if length > 0 then .[0].enabled == false else false end);
def versioning_not_enabled(rc): ((after(rc).versioning_configuration // []) | if length > 0 then .[0].status != "Enabled" else false end);
def bpa_weakened(rc):
  (after(rc) | [.block_public_acls, .block_public_policy, .ignore_public_acls, .restrict_public_buckets] | any(. == false));

def violations(rc):
  [
    (if ((guarded_types | index(rc.type)) != null) and is_delete(rc) then "destroys guarded " + rc.type + " (actions " + (actions(rc) | join(",")) + ")" else empty end),
    (if (rc.address | endswith("module.stage_guard.terraform_data.identity")) and is_delete(rc) then "destroys the state's stage identity (modules/stage-guard/v1)" else empty end),
    (if rc.type == "aws_dynamodb_table" and survives(rc) and pitr_off(rc) then "turns point-in-time recovery off" else empty end),
    (if rc.type == "aws_dynamodb_table" and survives(rc) and (after(rc).deletion_protection_enabled == false) then "turns deletion protection off" else empty end),
    (if rc.type == "aws_s3_bucket_versioning" and survives(rc) and versioning_not_enabled(rc) then "leaves bucket versioning not Enabled" else empty end),
    (if (rc.type == "aws_s3_bucket_public_access_block" or rc.type == "aws_s3_account_public_access_block") and survives(rc) and bpa_weakened(rc) then "weakens Block Public Access" else empty end)
  ];

plan as $plan
| if $plan == null then fail("BLOCKED: plan JSON missing or empty (did `terraform show -json` fail?)") else . end
| if ($plan | type) != "object" or ($plan.format_version | type) != "string" then fail("BLOCKED: input is not a Terraform plan JSON (no format_version)") else . end
| ($plan.resource_changes // []) as $changes
| [ $changes[] | . as $rc | violations($rc) as $v | select(($v | length) > 0)
    | { address: $rc.address, reasons: $v, allow: matching_allow($rc) } ] as $hits
| ($hits | map(select(.allow != null))) as $allowed
| ($hits | map(select(.allow == null))) as $blocked
| ($allowed[] | "ALLOWED by PR #\(.allow.pr // "?") until \(.allow.expires): \(.address): \(.reasons | join("; "))"),
  ($blocked[] | "BLOCKED: \(.address): \(.reasons | join("; "))"),
  (if ($blocked | length) > 0
   then fail("tf-plan-guard: \($blocked | length) blocked change(s); add a plan-guard-allow.json entry in the same PR or fix the plan")
   else "OK: tf-plan-guard found no blocked change (\($changes | length) resource changes, \($allowed | length) allowed)" end)
