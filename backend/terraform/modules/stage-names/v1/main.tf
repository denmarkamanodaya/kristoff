# stage-names v1 — the ONE place that builds every stage-scoped resource name, ARN and shared constant (core/dry.md:
# "one source per fact"). Pure: no resource, no provider, no data source. bootstrap/account, stacks/platform and
# stacks/compute all call it, so the IAM patterns in bootstrap and the resource names in the stacks cannot drift.
# The names follow docs/contract.md ("Storage", "Config", "Build artefacts").
# Trap: a name change here renames resources (replace) in every consumer; the plan guard blocks replacing the table
# and the bucket.

locals {
  prefix = "kristoff-${var.stage}"
  # Region/account scoped ARN prefix, e.g. arn:aws:dynamodb:ap-southeast-2:123456789012
  at = "${var.region}:${var.account_id}"

  table_name       = "${local.prefix}-core"
  table_arn        = "arn:aws:dynamodb:${local.at}:table/${local.table_name}"
  docs_bucket_name = "${local.prefix}-docs-${var.account_id}"
  docs_bucket_arn  = "arn:aws:s3:::${local.docs_bucket_name}"

  # SSM parameters (docs/contract.md "Config"). The three SecureStrings are created and set by the owner
  # (docs/runbooks/owner-setup.md), never by Terraform; ai-enabled is the String kill switch the platform stack owns.
  ssm_prefix                 = "/kristoff/${var.stage}"
  anthropic_key_param_name   = "${local.ssm_prefix}/anthropic-api-key"
  totp_secret_param_name     = "${local.ssm_prefix}/totp-secret"
  session_key_param_name     = "${local.ssm_prefix}/session-hmac-key"
  ai_enabled_param_name      = "${local.ssm_prefix}/ai-enabled"
  secret_param_names         = [local.anthropic_key_param_name, local.totp_secret_param_name, local.session_key_param_name]
  ssm_parameter_arn_prefix   = "arn:aws:ssm:${local.at}:parameter"
  ssm_parameter_arn_pattern  = "${local.ssm_parameter_arn_prefix}${local.ssm_prefix}/*"
  ai_enabled_param_arn       = "${local.ssm_parameter_arn_prefix}${local.ai_enabled_param_name}"
  secret_param_arns          = [for n in local.secret_param_names : "${local.ssm_parameter_arn_prefix}${n}"]
  lambda_function_arn_prefix = "arn:aws:lambda:${local.at}:function:${local.prefix}-"
  ci_role_path               = "/kristoff/ci/"
  lambda_role_path           = "/kristoff/lambda/"
  boundary_name              = "${local.prefix}-lambda-boundary"
  boundary_arn               = "arn:aws:iam::${var.account_id}:policy${local.ci_role_path}${local.boundary_name}"
  alerts_topic_name          = "${local.prefix}-alerts"
  alerts_topic_arn           = "arn:aws:sns:${local.at}:${local.alerts_topic_name}"
  amplify_app_name           = "${local.prefix}-web"
  amplify_app_arn_pattern    = "arn:aws:amplify:${local.at}:apps/*"
  tfstate_bucket_name        = "kristoff-tfstate-${var.account_id}"
  tfstate_key_prefix         = "kristoff/${var.stage}"
  cloudtrail_bucket_name     = "kristoff-cloudtrail-${var.account_id}"
  log_group_prefix           = "/kristoff/${var.stage}"
  docs_object_prefix         = "docs/"
  docs_objects_arn           = "${local.docs_bucket_arn}/${local.docs_object_prefix}*"

  # The only data actions a Lambda role may hold (backend/lambda-scope.json picks a subset per function). One source for
  # the compute stack's scope check and the bootstrap permission boundary, so the two cannot drift. Every name is an IAM
  # action in the AWS service reference (TransactWriteItems is not one: a transaction is authorised per item action).
  lambda_dynamodb_actions = [
    "dynamodb:GetItem", "dynamodb:BatchGetItem", "dynamodb:Query", "dynamodb:PutItem", "dynamodb:UpdateItem",
    "dynamodb:DeleteItem", "dynamodb:ConditionCheckItem",
  ]
  lambda_s3_actions = ["s3:GetObject", "s3:PutObject"]
}
