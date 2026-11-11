# roots/platform — outputs read by humans and the deploy workflow (through tf.sh <stage> platform output).

output "alerts_topic_arn" {
  description = "ARN of the alerts topic."
  value       = module.platform.alerts_topic_arn
}

output "table_name" {
  description = "DynamoDB table name."
  value       = module.platform.table_name
}

output "docs_bucket_name" {
  description = "Docs bucket name."
  value       = module.platform.docs_bucket_name
}

output "ai_enabled_param_name" {
  description = "SSM name of the AI kill switch."
  value       = module.platform.ai_enabled_param_name
}

output "secret_param_names" {
  description = "SSM names of the three owner-set SecureStrings."
  value       = module.platform.secret_param_names
}

output "amplify_app_id" {
  description = "Amplify app id (deploy workflow)."
  value       = module.platform.amplify_app_id
}

output "amplify_branch_name" {
  description = "Amplify branch the deployments go to (deploy workflow)."
  value       = module.platform.amplify_branch_name
}

output "web_origin" {
  description = "The one web origin and site URL, https://main.<Amplify default domain> (read by roots/compute through the platform state)."
  value       = module.platform.web_origin
}
