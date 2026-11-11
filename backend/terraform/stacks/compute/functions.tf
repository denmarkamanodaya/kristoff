# The functions (contract "Build artefacts" + "v2"): one execution role each, its statements derived from
# backend/lambda-scope.json (no wildcard action, no "*" resource). The permission boundary (bootstrap/account
# lambda_boundary.tf) is the ceiling above these. Bundles: backend/dist/<function key>.zip; a key without a bundle
# deploys the placeholder (503 NOT_DEPLOYED), so the wiring can be deployed before the backend is built.

locals {
  # HTTP functions: key -> routes (contract "Routes"). The authorizer has no route (api.tf wires it as the authorizer).
  http_functions = {
    authorizer        = { description = "HTTP API REQUEST authorizer: verifies the session token", routes = [], auth = "none" }
    "auth-verify"     = { description = "POST /v1/auth/verify: TOTP code -> session token", routes = ["POST /v1/auth/verify"], auth = "none" }
    "config-get"      = { description = "GET /v1/config", routes = ["GET /v1/config"], auth = "lambda" }
    "docs-list"       = { description = "GET /v1/docs", routes = ["GET /v1/docs"], auth = "lambda" }
    "docs-create"     = { description = "POST /v1/docs: doc record + presigned upload", routes = ["POST /v1/docs"], auth = "lambda" }
    "docs-confirm"    = { description = "POST /v1/docs/{id}/confirm", routes = ["POST /v1/docs/{id}/confirm"], auth = "lambda" }
    "docs-rename"     = { description = "PATCH /v1/docs/{id}", routes = ["PATCH /v1/docs/{id}"], auth = "lambda" }
    "docs-link"       = { description = "GET /v1/docs/{id}/link: presigned GET", routes = ["GET /v1/docs/{id}/link"], auth = "lambda" }
    "chats-list"      = { description = "GET /v1/chats", routes = ["GET /v1/chats"], auth = "lambda" }
    "chats-create"    = { description = "POST /v1/chats", routes = ["POST /v1/chats"], auth = "lambda" }
    "chats-get"       = { description = "GET /v1/chats/{id}", routes = ["GET /v1/chats/{id}"], auth = "lambda" }
    "chats-update"    = { description = "PATCH /v1/chats/{id}", routes = ["PATCH /v1/chats/{id}"], auth = "lambda" }
    "chats-delete"    = { description = "DELETE /v1/chats/{id}: thread + its messages", routes = ["DELETE /v1/chats/{id}"], auth = "lambda", timeout_s = 30 }
    "usage-get"       = { description = "GET /v1/usage", routes = ["GET /v1/usage"], auth = "lambda" }
    "projects-list"   = { description = "GET /v1/projects", routes = ["GET /v1/projects"], auth = "lambda" }
    "projects-create" = { description = "POST /v1/projects", routes = ["POST /v1/projects"], auth = "lambda" }
    "projects-rename" = { description = "PATCH /v1/projects/{id}", routes = ["PATCH /v1/projects/{id}"], auth = "lambda" }
    "projects-delete" = { description = "DELETE /v1/projects/{id}: ungroups its chats, then deletes it", routes = ["DELETE /v1/projects/{id}"], auth = "lambda", timeout_s = 30 }
    "docs-delete"     = { description = "DELETE /v1/docs/{id}: unlinks it from chats, deletes every object version, then the item", routes = ["DELETE /v1/docs/{id}"], auth = "lambda", timeout_s = 60 }
    # No route: docs-confirm invokes it asynchronously (contract v2 "Large PDFs"). async = true gives it a dead-letter
    # queue (failed invocations land there; alarms.tf watches it) and one Lambda retry.
    "docs-split" = { description = "Async: splits a large PDF into parts of <= 100 pages / 30 MB (invoked by docs-confirm)", routes = [], auth = "none", memory_mb = 3008, timeout_s = 300, async = true }
  }
  stream_function = "ask-stream"
  function_keys   = concat(keys(local.http_functions), [local.stream_function])

  # Each function's scope entry (backend/lambda-scope.json: dynamodb.table = actions on the table, dynamodb.gsi1 =
  # actions on index/GSI1, s3.actions on <bucket>/<s3.prefix><OWNER_ID>/*, ssm = SSM_* env keys it reads). A missing entry
  # or an unknown key reads as empty here so that scope_guard (scope.tf) reports the defect with its own message instead
  # of an index error; the guard then fails the plan.
  fn_scope = {
    for k in local.function_keys : k => {
      env        = try(local.scope.functions[k].env, [])
      table      = try(local.scope.functions[k].dynamodb.table, [])
      gsi1       = try(local.scope.functions[k].dynamodb.gsi1, [])
      s3_actions = try(local.scope.functions[k].s3.actions, [])
      s3_prefix  = try(local.scope.functions[k].s3.prefix, "")
      invoke     = try(local.scope.functions[k].lambda.invoke, [])
      ssm        = [for e in try(local.scope.functions[k].ssm, []) : e if contains(keys(local.ssm_env_arns), e)]
    }
  }

  # S3 actions split by resource type (stage-names): object actions on <bucket>/<prefix><OWNER_ID>/*, bucket-level
  # actions (ListBucketVersions) on the bucket ARN with the same prefix as an s3:prefix condition.
  s3_object_actions = { for k in local.function_keys : k => sort(setintersection(local.fn_scope[k].s3_actions, module.names.lambda_s3_object_actions)) }
  s3_bucket_actions = { for k in local.function_keys : k => sort(setintersection(local.fn_scope[k].s3_actions, module.names.lambda_s3_bucket_actions)) }

  # The alias every trigger and every function-to-function invoke targets (lambda-fn v1 always names it `live`).
  alias_name = "live"

  # One policy document per function, from its scope entry. Statements with no actions are left out.
  policies = {
    for k in local.function_keys : k => jsonencode({
      Version = "2012-10-17"
      Statement = concat(
        length(local.fn_scope[k].table) > 0 ? [{
          Sid = "TableItems", Effect = "Allow", Action = sort(local.fn_scope[k].table), Resource = [module.names.table_arn]
        }] : [],
        length(local.fn_scope[k].gsi1) > 0 ? [{
          Sid = "Gsi1Items", Effect = "Allow", Action = sort(local.fn_scope[k].gsi1), Resource = ["${module.names.table_arn}/index/GSI1"]
        }] : [],
        length(local.s3_object_actions[k]) > 0 ? [{
          Sid      = "DocObjects", Effect = "Allow", Action = local.s3_object_actions[k]
          Resource = ["${module.names.docs_bucket_arn}/${local.fn_scope[k].s3_prefix}${var.owner_id}/*"]
        }] : [],
        length(local.s3_bucket_actions[k]) > 0 ? [{
          Sid       = "DocVersionsList", Effect = "Allow", Action = local.s3_bucket_actions[k], Resource = [module.names.docs_bucket_arn]
          Condition = { StringLike = { "s3:prefix" = ["${local.fn_scope[k].s3_prefix}${var.owner_id}/*"] } }
        }] : [],
        length(local.fn_scope[k].invoke) > 0 ? [{
          Sid      = "InvokeFunctions", Effect = "Allow", Action = ["lambda:InvokeFunction"]
          Resource = sort([for f in local.fn_scope[k].invoke : "${module.names.lambda_function_arn_prefix}${f}:${local.alias_name}"])
        }] : [],
        length(local.fn_scope[k].ssm) > 0 ? [{
          Sid = "ReadParameters", Effect = "Allow", Action = ["ssm:GetParameter"], Resource = sort([for e in local.fn_scope[k].ssm : local.ssm_env_arns[e]])
        }] : [],
      )
    })
  }

  envs = { for k in local.function_keys : k => { for e in local.fn_scope[k].env : e => local.env_values[e] if contains(keys(local.env_values), e) } }

  # A key without a built bundle deploys the placeholder; the real bundle takes over the moment it exists (new hash).
  zip_paths = {
    for k in local.function_keys : k => (
      fileexists("${var.lambda_zip_dir}/${k}.zip") ? "${var.lambda_zip_dir}/${k}.zip" : "${path.module}/placeholder/stub.zip"
    )
  }
}

module "fn" {
  source   = "../../modules/lambda-fn/v1"
  for_each = local.http_functions

  name                     = "${module.names.prefix}-${each.key}"
  description              = each.value.description
  zip_path                 = local.zip_paths[each.key]
  memory_mb                = try(each.value.memory_mb, var.http_memory_mb)
  timeout_s                = try(each.value.timeout_s, var.http_timeout_s)
  dlq_enabled              = try(each.value.async, false)
  async_retry_attempts     = try(each.value.async, false) ? 1 : null
  env                      = local.envs[each.key]
  role_path                = module.names.lambda_role_path
  permissions_boundary_arn = module.names.boundary_arn
  policy_json              = local.policies[each.key]
  log_retention_days       = var.log_retention_days

  depends_on = [terraform_data.scope_guard]
}

# ask-stream: POST {STREAM_URL}/v1/chats/{id}/ask, streamed answer (contract "Stream"). The URL has no IAM auth; the
# function verifies the session token with the same use case as the authorizer.
module "ask" {
  source = "../../modules/lambda-stream-url/v1"

  name                     = "${module.names.prefix}-${local.stream_function}"
  description              = "POST /v1/chats/{id}/ask: streamed answer with citations (function URL)"
  zip_path                 = local.zip_paths[local.stream_function]
  memory_mb                = var.stream_memory_mb
  timeout_s                = var.stream_timeout_s
  env                      = local.envs[local.stream_function]
  role_path                = module.names.lambda_role_path
  permissions_boundary_arn = module.names.boundary_arn
  policy_json              = local.policies[local.stream_function]
  log_retention_days       = var.log_retention_days
  cors_allowed_origins     = [var.web_origin]

  depends_on = [terraform_data.scope_guard]
}
