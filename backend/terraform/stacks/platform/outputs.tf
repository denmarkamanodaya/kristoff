# platform stack — outputs read by humans and, through tf.sh <stage> platform output, by the deploy workflow.

output "alerts_topic_arn" {
  description = "ARN of the alerts topic."
  value       = aws_sns_topic.alerts.arn
}

output "table_name" {
  description = "DynamoDB table name."
  value       = module.table.table_name
}

output "docs_bucket_name" {
  description = "Docs bucket name."
  value       = module.docs_bucket.bucket_name
}

output "ai_enabled_param_name" {
  description = "SSM name of the AI kill switch (the owner flips it with aws ssm put-parameter --overwrite)."
  value       = aws_ssm_parameter.ai_enabled.name
}

output "secret_param_names" {
  description = "SSM names of the three owner-set SecureStrings (anthropic key, TOTP secret, session key); runbook step 'secrets'."
  value       = module.names.secret_param_names
}

output "amplify_app_id" {
  description = "Amplify app id (deploy workflow: aws amplify create-deployment)."
  value       = module.web.app_id
}

output "amplify_branch_name" {
  description = "Amplify branch the deployments go to."
  value       = module.web.branch_name
}

output "web_origin" {
  description = "The one web origin, https://main.<Amplify default domain>: the site URL and the only origin every CORS rule admits."
  value       = module.web.branch_url
}
