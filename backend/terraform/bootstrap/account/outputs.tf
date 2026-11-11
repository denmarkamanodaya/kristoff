# Bootstrap account module — outputs the owner copies into GitHub and (for the state bucket) checks against the env files.

output "tfstate_bucket" {
  description = "State bucket name; env/<stage>.<stack>.backend.hcl carries the same name."
  value       = aws_s3_bucket.tfstate.id
}

output "ci_plan_role_arn" {
  description = "Role ARN for pull_request plans -> GitHub repository variable AWS_PLAN_ROLE_ARN_<STAGE> (unused until a plan job exists)."
  value       = aws_iam_role.ci_plan.arn
}

output "ci_deploy_role_arn" {
  description = "Role ARN for deploys -> GitHub environment variable AWS_DEPLOY_ROLE_ARN on environment <stage>."
  value       = aws_iam_role.ci_deploy.arn
}

output "lambda_boundary_arn" {
  description = "Permission boundary ARN every Lambda role must carry (the compute stack builds the same ARN by name)."
  value       = aws_iam_policy.lambda_boundary.arn
}

output "cloudtrail_arn" {
  description = "ARN of the management-events trail."
  value       = aws_cloudtrail.management.arn
}

output "oidc_provider_arn" {
  description = "ARN of the GitHub Actions OIDC provider."
  value       = aws_iam_openid_connect_provider.github.arn
}
