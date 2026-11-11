# HTTP API: Lambda REQUEST authorizer on every route but POST /v1/auth/verify, Lambda proxy integrations on the functions'
# `live` alias, one invoke permission per route plus one for the authorizer. Stage throttling and CORS: contract.

locals {
  routes = flatten([
    for fk, f in local.http_functions : [
      for r in f.routes : {
        function = fk
        key      = r
        auth     = f.auth
        # The route's method and path in an execute-api ARN; a path parameter ({id}) becomes *.
        arn_path = replace(replace(r, " ", ""), "/\\{[^}]+\\}/", "*")
      }
    ]
  ])
}

module "api" {
  source = "../../modules/http-api/v1"

  name                   = "${module.names.prefix}-api"
  description            = "Kristoff ${var.stage} API: session-token authorizer on every route but auth/verify"
  throttling_rate_limit  = var.api_throttling_rate_limit
  throttling_burst_limit = var.api_throttling_burst_limit
  access_log_group_name  = "${module.names.log_group_prefix}/api-access"
  authorizer_invoke_arn  = module.fn["authorizer"].alias_invoke_arn
  cors_allowed_origins   = [var.web_origin]

  routes = [
    for r in local.routes : {
      route_key         = r.key
      target_invoke_arn = module.fn[r.function].alias_invoke_arn
      auth              = r.auth
    }
  ]
}

resource "aws_lambda_permission" "api" {
  for_each = { for r in local.routes : r.key => r }

  statement_id  = "AllowApi-${replace(each.key, "/[^A-Za-z0-9]+/", "-")}"
  action        = "lambda:InvokeFunction"
  function_name = module.fn[each.value.function].function_name
  qualifier     = module.fn[each.value.function].alias_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${module.api.execution_arn}/*/${each.value.arn_path}"
}

# API Gateway invokes the authorizer as arn:.../authorizers/<id> (API Gateway developer guide, Lambda authorizers).
resource "aws_lambda_permission" "authorizer" {
  statement_id  = "AllowApiAuthorizer"
  action        = "lambda:InvokeFunction"
  function_name = module.fn["authorizer"].function_name
  qualifier     = module.fn["authorizer"].alias_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${module.api.execution_arn}/authorizers/${module.api.authorizer_id}"
}
