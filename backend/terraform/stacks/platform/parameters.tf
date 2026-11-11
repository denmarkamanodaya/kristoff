# SSM parameters (docs/contract.md "Config").
# - ai-enabled: String `true` | `false`, the kill switch the ask Lambda reads (60 s cache). Terraform creates it as `true`
#   and then IGNORES its value: the owner flips it with `aws ssm put-parameter --overwrite`, and a deploy must not revert
#   an emergency stop. A destroy/recreate would reset it, hence prevent_destroy.
# - The three SecureStrings (anthropic-api-key, totp-secret, session-hmac-key) are NOT created here: the AWS provider
#   reads a parameter back with WithDecryption=true on every refresh, so a Terraform-managed secret would sit in the state
#   and the deploy role would need to decrypt it. The owner creates them before the first deploy
#   (docs/runbooks/owner-setup.md); Terraform passes only their names (stack/terraform.md).

resource "aws_ssm_parameter" "ai_enabled" {
  name        = module.names.ai_enabled_param_name
  description = "${module.names.prefix} AI kill switch: false refuses every question with AI_DISABLED. Owner-flipped; Terraform ignores the value."
  type        = "String"
  value       = "true"

  tags = { Name = module.names.ai_enabled_param_name }

  lifecycle {
    prevent_destroy = true
    ignore_changes  = [value]
  }
}
