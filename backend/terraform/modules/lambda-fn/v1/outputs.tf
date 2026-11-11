# lambda-fn v1 — outputs. `granted_actions` and `env_keys` let a stack assert a function's scope at plan time.

output "function_name" {
  description = "Function name."
  value       = aws_lambda_function.this.function_name
}

output "function_arn" {
  description = "Unqualified function ARN."
  value       = aws_lambda_function.this.arn
}

output "published_version" {
  description = "Version number published by this apply."
  value       = aws_lambda_function.this.version
}

output "alias_arn" {
  description = "ARN of the `live` alias (what triggers must target)."
  value       = aws_lambda_alias.live.arn
}

output "alias_invoke_arn" {
  description = "Invoke ARN of the `live` alias for API Gateway integrations."
  value       = aws_lambda_alias.live.invoke_arn
}

output "alias_name" {
  description = "Name of the alias (`live`), the qualifier for aws_lambda_permission."
  value       = aws_lambda_alias.live.name
}

output "role_arn" {
  description = "Execution role ARN."
  value       = aws_iam_role.this.arn
}

output "role_name" {
  description = "Execution role name."
  value       = aws_iam_role.this.name
}

output "log_group_name" {
  description = "CloudWatch log group name."
  value       = aws_cloudwatch_log_group.this.name
}

output "dlq_arn" {
  description = "Dead-letter queue ARN, or null when `dlq_enabled` is false."
  value       = one(aws_sqs_queue.dlq[*].arn)
}

output "granted_actions" {
  description = "Sorted, distinct IAM actions allowed by the role's inline policy (module statements + policy_json); known at plan time."
  value       = sort(distinct(concat(local.logs_actions, local.xray_actions, local.dlq_actions, local.caller_actions)))
}

output "env_keys" {
  description = "Sorted environment variable keys set on the function."
  value       = sort(keys(var.env))
}

output "async_retry_attempts" {
  description = "Retries of a failed asynchronous invocation of the `live` alias, or null when Lambda's default is left unmanaged."
  value       = one(aws_lambda_function_event_invoke_config.live[*].maximum_retry_attempts)
}
