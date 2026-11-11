# Account-level hardening: S3 Block Public Access, password policy, GuardDuty (prod).
# Trap: a new member account has no account-level BPA set — nothing is inherited.

# account_id is left unset on purpose: it defaults to the caller's account, which the root precondition has already
# matched against the literal.
resource "aws_s3_account_public_access_block" "this" {
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Moot without IAM users (the SCP forbids creating them) — cheap insurance.
resource "aws_iam_account_password_policy" "this" {
  minimum_password_length        = 14
  require_lowercase_characters   = true
  require_uppercase_characters   = true
  require_numbers                = true
  require_symbols                = true
  password_reuse_prevention      = 24
  allow_users_to_change_password = true
  hard_expiry                    = false
}

resource "aws_guardduty_detector" "this" {
  count = var.guardduty_enabled ? 1 : 0

  enable                       = true
  finding_publishing_frequency = "FIFTEEN_MINUTES"
  tags                         = { Name = "${local.name_prefix}-guardduty" }
}
