# http-api v1 — plan-mode tests, no AWS calls.

provider "aws" {
  region                      = "ap-southeast-2"
  access_key                  = "test"
  secret_key                  = "test"
  skip_credentials_validation = true
  skip_requesting_account_id  = true
  skip_metadata_api_check     = true
  skip_region_validation      = true
}

variables {
  name                   = "kristoff-prod-api"
  throttling_rate_limit  = 10
  throttling_burst_limit = 20
  access_log_group_name  = "/kristoff/prod/api-access"
  authorizer_invoke_arn  = "arn:aws:apigateway:ap-southeast-2:lambda:path/2015-03-31/functions/arn:aws:lambda:ap-southeast-2:123456789012:function:kristoff-prod-authorizer:live/invocations"
  cors_allowed_origins   = ["https://kristoff.example.dev"]
  routes = [
    {
      route_key         = "POST /v1/auth/verify"
      target_invoke_arn = "arn:aws:apigateway:ap-southeast-2:lambda:path/2015-03-31/functions/arn:aws:lambda:ap-southeast-2:123456789012:function:kristoff-prod-auth-verify:live/invocations"
      auth              = "none"
    },
    {
      route_key              = "GET /v1/config"
      target_invoke_arn      = "arn:aws:apigateway:ap-southeast-2:lambda:path/2015-03-31/functions/arn:aws:lambda:ap-southeast-2:123456789012:function:kristoff-prod-config-get:live/invocations"
      auth                   = "lambda"
      throttling_rate_limit  = 5
      throttling_burst_limit = 10
    },
  ]
}

run "lambda_routes_use_the_request_authorizer" {
  command = plan

  assert {
    condition     = aws_apigatewayv2_route.this["GET /v1/config"].authorization_type == "CUSTOM" && aws_apigatewayv2_route.this["POST /v1/auth/verify"].authorization_type == "NONE"
    error_message = "lambda routes are CUSTOM, none routes are NONE"
  }
  assert {
    condition     = aws_apigatewayv2_authorizer.lambda[0].authorizer_type == "REQUEST" && aws_apigatewayv2_authorizer.lambda[0].enable_simple_responses == true && aws_apigatewayv2_authorizer.lambda[0].authorizer_payload_format_version == "2.0"
    error_message = "the authorizer is a REQUEST authorizer with simple responses (payload 2.0)"
  }
  assert {
    condition     = aws_apigatewayv2_authorizer.lambda[0].authorizer_result_ttl_in_seconds == 0 && tolist(aws_apigatewayv2_authorizer.lambda[0].identity_sources) == tolist(["$request.header.Authorization"])
    error_message = "no authorizer cache; the token comes from the Authorization header (contract Build artefacts)"
  }
}

run "cors_lists_exact_origins" {
  command = plan

  assert {
    condition     = tolist(aws_apigatewayv2_api.this.cors_configuration[0].allow_origins) == tolist(["https://kristoff.example.dev"]) && toset(aws_apigatewayv2_api.this.cors_configuration[0].allow_headers) == toset(["authorization", "content-type"])
    error_message = "CORS allows exactly the web origins and the two request headers"
  }
  assert {
    condition     = toset(aws_apigatewayv2_api.this.cors_configuration[0].expose_headers) == toset(["retry-after"])
    error_message = "the browser may read Retry-After (contract Clarifications)"
  }
}

run "stage_throttles_and_logs" {
  command = plan

  assert {
    condition     = aws_apigatewayv2_stage.default.default_route_settings[0].throttling_rate_limit == 10 && aws_apigatewayv2_stage.default.default_route_settings[0].throttling_burst_limit == 20
    error_message = "stage defaults come from the caller"
  }
  assert {
    condition     = one([for rs in aws_apigatewayv2_stage.default.route_settings : rs if rs.route_key == "GET /v1/config"]).throttling_rate_limit == 5
    error_message = "a route can carry its own throttle"
  }
  assert {
    condition     = strcontains(aws_apigatewayv2_stage.default.access_log_settings[0].format, "$context.requestId") && !strcontains(aws_apigatewayv2_stage.default.access_log_settings[0].format, "$context.path") && !strcontains(aws_apigatewayv2_stage.default.access_log_settings[0].format, "$context.requestBody")
    error_message = "access log carries the request id and never the path with query or a body"
  }
  assert {
    condition     = aws_cloudwatch_log_group.access.retention_in_days == 30
    error_message = "access logs keep 30 days"
  }
  assert {
    condition     = aws_apigatewayv2_integration.lambda["GET /v1/config"].payload_format_version == "2.0" && aws_apigatewayv2_integration.lambda["GET /v1/config"].integration_type == "AWS_PROXY"
    error_message = "integrations are Lambda proxy v2.0"
  }
  assert {
    condition     = output.route_keys == tolist(["GET /v1/config", "POST /v1/auth/verify"])
    error_message = "route_keys lists the routes sorted"
  }
}

run "public_routes_without_authorizer" {
  command = plan

  variables {
    lambda_authorizer_enabled = false
    authorizer_invoke_arn     = ""
    routes = [
      {
        route_key         = "POST /v1/auth/verify"
        target_invoke_arn = "arn:aws:apigateway:ap-southeast-2:lambda:path/2015-03-31/functions/arn:aws:lambda:ap-southeast-2:123456789012:function:kristoff-prod-auth-verify:live/invocations"
        auth              = "none"
      },
    ]
  }

  assert {
    condition     = length(aws_apigatewayv2_authorizer.lambda) == 0 && output.authorizer_id == null
    error_message = "no authorizer when disabled"
  }
}

run "rejects_lambda_route_without_authorizer" {
  command = plan

  variables {
    lambda_authorizer_enabled = false
    authorizer_invoke_arn     = ""
  }

  expect_failures = [var.routes]
}

run "rejects_wildcard_origin" {
  command = plan

  variables {
    cors_allowed_origins = ["*"]
  }

  expect_failures = [var.cors_allowed_origins]
}

run "rejects_duplicate_route_keys" {
  command = plan

  variables {
    routes = [
      {
        route_key         = "GET /v1/config"
        target_invoke_arn = "arn:aws:apigateway:ap-southeast-2:lambda:path/2015-03-31/functions/arn:aws:lambda:ap-southeast-2:123456789012:function:a:live/invocations"
        auth              = "lambda"
      },
      {
        route_key         = "GET /v1/config"
        target_invoke_arn = "arn:aws:apigateway:ap-southeast-2:lambda:path/2015-03-31/functions/arn:aws:lambda:ap-southeast-2:123456789012:function:b:live/invocations"
        auth              = "lambda"
      },
    ]
  }

  expect_failures = [var.routes]
}
