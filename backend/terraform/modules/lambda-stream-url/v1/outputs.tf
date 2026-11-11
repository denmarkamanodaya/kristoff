# lambda-stream-url v1 — outputs. `granted_actions` and `env_keys` let a stack assert the function's scope at plan time.

output "function_name" {
  description = "Function name."
  value       = module.fn.function_name
}

output "function_url" {
  description = "Public HTTPS URL of the `live` alias (https://<url id>.lambda-url.<region>.on.aws/), ending in /."
  value       = aws_lambda_function_url.this.function_url
}

output "alias_arn" {
  description = "ARN of the `live` alias the URL serves."
  value       = module.fn.alias_arn
}

output "role_name" {
  description = "Execution role name."
  value       = module.fn.role_name
}

output "log_group_name" {
  description = "CloudWatch log group name."
  value       = module.fn.log_group_name
}

output "granted_actions" {
  description = "Sorted, distinct IAM actions the role's inline policy allows; known at plan time."
  value       = module.fn.granted_actions
}

output "env_keys" {
  description = "Sorted environment variable keys set on the function."
  value       = module.fn.env_keys
}
