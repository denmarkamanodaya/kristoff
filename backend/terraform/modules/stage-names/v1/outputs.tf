# stage-names v1 — outputs. One per fact in main.tf; consumers never rebuild a name.

output "prefix" {
  description = "Resource name prefix kristoff-<stage>."
  value       = local.prefix
}

output "table_name" {
  description = "DynamoDB table name kristoff-<stage>-core (docs/contract.md \"Storage\")."
  value       = local.table_name
}

output "table_arn" {
  description = "DynamoDB table ARN (append /index/* for its indexes)."
  value       = local.table_arn
}

output "docs_bucket_name" {
  description = "Private PDF bucket kristoff-<stage>-docs-<account id>."
  value       = local.docs_bucket_name
}

output "docs_bucket_arn" {
  description = "Docs bucket ARN."
  value       = local.docs_bucket_arn
}

output "docs_objects_arn" {
  description = "ARN pattern of every PDF object, <bucket ARN>/docs/* (keys are docs/<uid>/<docId>.pdf, server-built)."
  value       = local.docs_objects_arn
}

output "anthropic_key_param_name" {
  description = "SSM SecureString name of the Anthropic API key (owner-set, never in Terraform)."
  value       = local.anthropic_key_param_name
}

output "totp_secret_param_name" {
  description = "SSM SecureString name of the base32 TOTP secret (owner-set, never in Terraform)."
  value       = local.totp_secret_param_name
}

output "session_key_param_name" {
  description = "SSM SecureString name of the session HMAC key (owner-set, never in Terraform)."
  value       = local.session_key_param_name
}

output "ai_enabled_param_name" {
  description = "SSM String name of the AI kill switch (true | false; Terraform creates it, the owner flips it)."
  value       = local.ai_enabled_param_name
}

output "ai_enabled_param_arn" {
  description = "ARN of the AI kill switch parameter."
  value       = local.ai_enabled_param_arn
}

output "secret_param_names" {
  description = "Names of the three owner-set SecureString parameters, in the order anthropic key, TOTP secret, session key."
  value       = local.secret_param_names
}

output "secret_param_arns" {
  description = "ARNs of the three owner-set SecureString parameters (same order as secret_param_names)."
  value       = local.secret_param_arns
}

output "ssm_parameter_arn_prefix" {
  description = "Prefix that turns an SSM name (with its leading slash) into its ARN: arn:aws:ssm:<region>:<account>:parameter."
  value       = local.ssm_parameter_arn_prefix
}

output "ssm_parameter_arn_pattern" {
  description = "ARN pattern of every SSM parameter of the stage (/kristoff/<stage>/*)."
  value       = local.ssm_parameter_arn_pattern
}

output "lambda_function_arn_prefix" {
  description = "ARN prefix of every function of the stage (append * or a function key)."
  value       = local.lambda_function_arn_prefix
}

output "ci_role_path" {
  description = "IAM path of the CI roles and the boundary policy."
  value       = local.ci_role_path
}

output "lambda_role_path" {
  description = "IAM path of every Lambda execution role (the deploy role may only manage this path)."
  value       = local.lambda_role_path
}

output "boundary_name" {
  description = "Name of the Lambda permission boundary policy."
  value       = local.boundary_name
}

output "boundary_arn" {
  description = "ARN of the Lambda permission boundary policy."
  value       = local.boundary_arn
}

output "alerts_topic_name" {
  description = "Alerts SNS topic name."
  value       = local.alerts_topic_name
}

output "alerts_topic_arn" {
  description = "Alerts SNS topic ARN."
  value       = local.alerts_topic_arn
}

output "amplify_app_name" {
  description = "Amplify Hosting app name of the web client."
  value       = local.amplify_app_name
}

output "amplify_app_arn_pattern" {
  description = "ARN pattern of Amplify apps in the account (app ids are random; the account hosts one stage)."
  value       = local.amplify_app_arn_pattern
}

output "tfstate_bucket_name" {
  description = "Terraform state bucket of the account."
  value       = local.tfstate_bucket_name
}

output "tfstate_key_prefix" {
  description = "State key prefix of the stage: <prefix>/<stack>.tfstate."
  value       = local.tfstate_key_prefix
}

output "cloudtrail_bucket_name" {
  description = "CloudTrail log bucket of the account."
  value       = local.cloudtrail_bucket_name
}

output "log_group_prefix" {
  description = "Prefix of non-Lambda log groups (API access logs)."
  value       = local.log_group_prefix
}

output "lambda_dynamodb_actions" {
  description = "DynamoDB item actions a Lambda role may hold on the table and its indexes (the boundary's ceiling)."
  value       = local.lambda_dynamodb_actions
}

output "lambda_s3_object_actions" {
  description = "S3 object actions a Lambda role may hold on docs/* objects of the docs bucket (the boundary's ceiling)."
  value       = local.lambda_s3_object_actions
}

output "lambda_s3_bucket_actions" {
  description = "S3 bucket-level actions a Lambda role may hold on the docs bucket, always with an s3:prefix condition under docs/ (the boundary's ceiling)."
  value       = local.lambda_s3_bucket_actions
}

output "lambda_dlq_arn_pattern" {
  description = "ARN pattern of the Lambda dead-letter queues, <prefix>-<function key>-dlq (the boundary admits sqs:SendMessage on them)."
  value       = local.lambda_dlq_arn_pattern
}

output "docs_object_prefix" {
  description = "Key prefix of every PDF object, docs/ (keys are docs/<uid>/<docId>.pdf)."
  value       = local.docs_object_prefix
}
