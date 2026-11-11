# lambda-stream-url v1 — one streaming Lambda function behind a public function URL: the function itself (role, inline
# policy, log group, published version, `live` alias) comes from lambda-fn v1; this module adds the URL on the `live`
# alias with `invoke_mode = RESPONSE_STREAM`, CORS for exact origins, and the two resource-policy statements a public
# URL needs.
# Traps: `authorization_type = "NONE"` means anyone can reach the URL; the function must authenticate every request
# itself (Kristoff: the ask Lambda verifies the session token, docs/contract.md "Auth flow"). Since October 2025 a
# function URL needs BOTH lambda:InvokeFunctionUrl and lambda:InvokeFunction (the latter limited to calls made through
# the URL with `invoked_via_function_url`); missing either answers 403. The URL is on the alias, so a rollback of the
# alias rolls the URL back with it.

module "fn" {
  source = "../../lambda-fn/v1"

  name                         = var.name
  description                  = var.description
  zip_path                     = var.zip_path
  handler                      = var.handler
  runtime                      = var.runtime
  architecture                 = var.architecture
  memory_mb                    = var.memory_mb
  timeout_s                    = var.timeout_s
  env                          = var.env
  role_path                    = var.role_path
  permissions_boundary_arn     = var.permissions_boundary_arn
  policy_json                  = var.policy_json
  log_retention_days           = var.log_retention_days
  reserved_concurrency_enabled = var.reserved_concurrency_enabled
  reserved_concurrency         = var.reserved_concurrency
  tags                         = var.tags
}

resource "aws_lambda_function_url" "this" {
  function_name      = module.fn.function_name
  qualifier          = module.fn.alias_name
  authorization_type = "NONE"
  invoke_mode        = "RESPONSE_STREAM"

  cors {
    allow_origins  = var.cors_allowed_origins
    allow_methods  = ["POST"]
    allow_headers  = ["authorization", "content-type"]
    expose_headers = ["retry-after"]
    max_age        = 3600
  }
}

resource "aws_lambda_permission" "url" {
  statement_id           = "AllowPublicFunctionUrl"
  action                 = "lambda:InvokeFunctionUrl"
  function_name          = module.fn.function_name
  qualifier              = module.fn.alias_name
  principal              = "*"
  function_url_auth_type = "NONE"
}

resource "aws_lambda_permission" "invoke_via_url" {
  statement_id             = "AllowInvokeThroughFunctionUrl"
  action                   = "lambda:InvokeFunction"
  function_name            = module.fn.function_name
  qualifier                = module.fn.alias_name
  principal                = "*"
  invoked_via_function_url = true
}
