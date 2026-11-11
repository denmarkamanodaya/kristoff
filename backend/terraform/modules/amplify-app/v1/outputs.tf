# amplify-app v1 — outputs. The deploy workflow needs app_id + branch_name; CORS needs branch_url (the web origin).

output "app_id" {
  description = "Amplify app id (aws amplify create-deployment --app-id)."
  value       = aws_amplify_app.this.id
}

output "branch_name" {
  description = "Branch the deployments go to."
  value       = aws_amplify_branch.this.branch_name
}

output "default_domain" {
  description = "Amplify default domain of the app; the branch is served at https://<branch>.<default_domain>."
  value       = aws_amplify_app.this.default_domain
}

output "branch_url" {
  description = "HTTPS origin of the branch on the Amplify default domain, https://<branch>.<default domain> (no trailing slash): the web origin."
  value       = "https://${aws_amplify_branch.this.branch_name}.${aws_amplify_app.this.default_domain}"
}
