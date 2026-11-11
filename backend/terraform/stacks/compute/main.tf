# compute stack — the 14 Lambda functions of Kristoff, the HTTP API with its Lambda REQUEST authorizer, the streaming
# function URL of ask-stream, and the alarms. Composition root of the compute layer: it only wires modules. Platform
# resources (table, docs bucket, SSM parameters, alerts topic) are addressed by NAME through stage-names, never through
# remote state. Names, routes, timeouts and throttles: docs/contract.md ("Routes", "Build artefacts").
#
# Per-function scope comes from backend/lambda-scope.json (written by the backend build; shape in
# tests/fixtures/lambda-scope.json): the env keys each bundle reads, its DynamoDB and S3 actions and the SSM parameters
# it reads. Every IAM grant and env map below is derived from that file, and scope_guard (scope.tf) fails the plan
# when the file names an unknown function, env key or action.

module "names" {
  source = "../../modules/stage-names/v1"

  stage      = var.stage
  account_id = var.account_id
  region     = var.region
}

locals {
  scope = jsondecode(file(var.lambda_scope_path))

  # Every env key the backend's Zod env schema knows (backend/src/composition/env.ts; contract "Config"), with its value.
  # A function receives only the keys its scope lists.
  env_values = {
    STAGE                = var.stage
    OWNER_ID             = var.owner_id
    TABLE_NAME           = module.names.table_name
    DOCS_BUCKET          = module.names.docs_bucket_name
    WEB_ORIGINS          = var.web_origin
    MODEL_IDS            = join(",", var.model_ids)
    MAX_TOKENS           = tostring(var.max_tokens)
    MAX_ATTACHED_DOCS    = tostring(var.max_attached_docs)
    MAX_HISTORY_TURNS    = tostring(var.max_history_turns)
    DAILY_QUESTION_CAP   = tostring(var.daily_question_cap)
    SESSION_TTL_SECONDS  = tostring(var.session_ttl_seconds)
    ANTHROPIC_TIMEOUT_MS = tostring(var.anthropic_timeout_ms)
    SSM_ANTHROPIC_KEY    = module.names.anthropic_key_param_name
    SSM_TOTP_SECRET      = module.names.totp_secret_param_name
    SSM_SESSION_KEY      = module.names.session_key_param_name
    SSM_AI_ENABLED       = module.names.ai_enabled_param_name
  }

  # The SSM_* env keys and the parameter ARN each one names.
  ssm_env_arns = {
    SSM_ANTHROPIC_KEY = "${module.names.ssm_parameter_arn_prefix}${module.names.anthropic_key_param_name}"
    SSM_TOTP_SECRET   = "${module.names.ssm_parameter_arn_prefix}${module.names.totp_secret_param_name}"
    SSM_SESSION_KEY   = "${module.names.ssm_parameter_arn_prefix}${module.names.session_key_param_name}"
    SSM_AI_ENABLED    = module.names.ai_enabled_param_arn
  }
}
