# http-api v1 — outputs. The stack builds Lambda invoke permissions from `execution_arn` and `authorizer_id`.

output "api_id" {
  description = "API id."
  value       = aws_apigatewayv2_api.this.id
}

output "api_endpoint" {
  description = "Default execute-api endpoint (https://<id>.execute-api.<region>.amazonaws.com)."
  value       = aws_apigatewayv2_api.this.api_endpoint
}

output "execution_arn" {
  description = "Execution ARN prefix for aws_lambda_permission source_arn (\"<arn>/*/<METHOD>/<path>\", \"<arn>/authorizers/<id>\")."
  value       = aws_apigatewayv2_api.this.execution_arn
}

output "authorizer_id" {
  description = "Id of the Lambda REQUEST authorizer, or null when it is disabled."
  value       = one(aws_apigatewayv2_authorizer.lambda[*].id)
}

output "stage_name" {
  description = "Stage name ($default)."
  value       = aws_apigatewayv2_stage.default.name
}

output "access_log_group_name" {
  description = "Access-log group name."
  value       = aws_cloudwatch_log_group.access.name
}

output "route_keys" {
  description = "Sorted route keys created."
  value       = sort(keys(local.routes))
}
