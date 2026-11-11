# http-api v1 — one HTTP API ($default stage, auto-deploy), access logs, stage + per-route throttles, CORS for exact
# browser origins, an optional Lambda REQUEST authorizer (simple responses) and a Lambda proxy integration per route.
# Traps: the module never writes Lambda invoke permissions (the stack does, for the routes AND the authorizer). With
# `authorizer_result_ttl_in_seconds = 0` nothing is cached, so every request reaches the authorizer; a request without
# the identity source header is refused with 401 before the authorizer runs.

locals {
  routes = { for r in var.routes : r.route_key => r }
}

resource "aws_apigatewayv2_api" "this" {
  name          = var.name
  description   = var.description
  protocol_type = "HTTP"

  cors_configuration {
    allow_origins  = var.cors_allowed_origins
    allow_methods  = var.cors_allowed_methods
    allow_headers  = var.cors_allowed_headers
    expose_headers = ["retry-after"]
    max_age        = 3600
  }

  tags = merge(var.tags, { Name = var.name })
}

resource "aws_cloudwatch_log_group" "access" {
  name              = var.access_log_group_name
  retention_in_days = var.access_log_retention_days
  tags              = merge(var.tags, { Name = var.access_log_group_name })
}

resource "aws_apigatewayv2_authorizer" "lambda" {
  count = var.lambda_authorizer_enabled ? 1 : 0

  api_id                            = aws_apigatewayv2_api.this.id
  name                              = "${var.name}-authorizer"
  authorizer_type                   = "REQUEST"
  authorizer_uri                    = var.authorizer_invoke_arn
  authorizer_payload_format_version = "2.0"
  enable_simple_responses           = true
  authorizer_result_ttl_in_seconds  = 0
  identity_sources                  = ["$request.header.Authorization"]
}

resource "aws_apigatewayv2_integration" "lambda" {
  for_each = local.routes

  api_id                 = aws_apigatewayv2_api.this.id
  integration_type       = "AWS_PROXY"
  integration_uri        = each.value.target_invoke_arn
  integration_method     = "POST"
  payload_format_version = "2.0"
  timeout_milliseconds   = 30000
}

resource "aws_apigatewayv2_route" "this" {
  for_each = local.routes

  api_id             = aws_apigatewayv2_api.this.id
  route_key          = each.key
  target             = "integrations/${aws_apigatewayv2_integration.lambda[each.key].id}"
  authorization_type = each.value.auth == "lambda" ? "CUSTOM" : "NONE"
  authorizer_id      = each.value.auth == "lambda" ? aws_apigatewayv2_authorizer.lambda[0].id : null
}

resource "aws_apigatewayv2_stage" "default" {
  api_id      = aws_apigatewayv2_api.this.id
  name        = "$default"
  auto_deploy = true

  # route_settings name routes by key, so the stage is created after them.
  depends_on = [aws_apigatewayv2_route.this]

  access_log_settings {
    destination_arn = aws_cloudwatch_log_group.access.arn
    format          = var.access_log_format
  }

  default_route_settings {
    throttling_rate_limit  = var.throttling_rate_limit
    throttling_burst_limit = var.throttling_burst_limit
  }

  dynamic "route_settings" {
    for_each = { for k, r in local.routes : k => r if r.throttling_rate_limit != null }
    content {
      route_key              = route_settings.key
      throttling_rate_limit  = route_settings.value.throttling_rate_limit
      throttling_burst_limit = coalesce(route_settings.value.throttling_burst_limit, route_settings.value.throttling_rate_limit * 2)
    }
  }

  tags = merge(var.tags, { Name = "${var.name}-default" })
}
