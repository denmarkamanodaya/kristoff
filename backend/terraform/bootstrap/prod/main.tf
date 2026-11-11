# Bootstrap root for the kristoff-prod account — OWNER-RUN ONCE with the SSO profile kristoff-prod.
# Local state on purpose: terraform.tfstate stays here, git-ignored; back it up outside git after each apply (it holds
# ARNs and names only). docs/runbooks/owner-setup.md is the step-by-step.
# Traps: the preconditions below fail the plan on a malformed literal or foreign credentials; run from
# THIS directory; never -var, -var-file, -backend-config or -reconfigure; CI never runs this stack; an agent never
# applies it (stack/terraform.md): the owner reads the plan first.
#
#   aws sso login --sso-session dk
#   aws sts get-caller-identity --profile kristoff-prod      # must print the kristoff-prod account id
#   AWS_PROFILE=kristoff-prod terraform init
#   AWS_PROFILE=kristoff-prod terraform plan -out=bootstrap.tfplan
#   AWS_PROFILE=kristoff-prod terraform apply bootstrap.tfplan

terraform {
  required_version = "~> 1.14"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.67"
    }
  }
}

locals {
  stage           = "prod"
  region          = "ap-southeast-2"
  account_id      = "815882254506" # kristoff-prod in OU kristoff (ou-2con-ekrgjz2b)
  github_owner    = "denmarkamanodaya"
  github_owner_id = "5304296" # gh api users/denmarkamanodaya --jq .id
  github_repo     = "kristoff"
  github_repo_id  = "1454246629"                 # gh api repos/denmarkamanodaya/kristoff --jq .id
  alert_email     = "denmarkamanodaya@gmail.com" # owner mailbox for Budget notifications
}

provider "aws" {
  region = local.region
  default_tags {
    tags = {
      Project   = "kristoff"
      Stage     = local.stage
      Stack     = "bootstrap"
      ManagedBy = "terraform"
      Repo      = "${local.github_owner}/${local.github_repo}"
    }
  }
}

data "aws_caller_identity" "current" {}

# Fails the plan while a TODO marker is still in place or the credentials belong to another account.
resource "terraform_data" "literal_guard" {
  lifecycle {
    precondition {
      condition     = can(regex("^[0-9]{12}$", local.account_id))
      error_message = "bootstrap/prod: replace the TODO account id with the 12-digit kristoff-prod account id."
    }
    precondition {
      condition     = can(regex("^[0-9]+$", local.github_repo_id))
      error_message = "bootstrap/prod: github_repo_id must be the numeric repository id (create the GitHub repo first)."
    }
    precondition {
      condition     = can(regex("^[^@[:space:]]+@[^@[:space:]]+\\.[^@[:space:]]+$", local.alert_email))
      error_message = "bootstrap/prod: alert_email must be a valid e-mail address (owner mailbox)."
    }
    precondition {
      condition     = data.aws_caller_identity.current.account_id == local.account_id
      error_message = "bootstrap/prod is the kristoff-prod account; the current credentials belong to another account (check AWS_PROFILE)."
    }
  }
}

module "account" {
  source = "../account"

  stage             = local.stage
  account_id        = local.account_id
  region            = local.region
  github_owner      = local.github_owner
  github_owner_id   = local.github_owner_id
  github_repo       = local.github_repo
  github_repo_id    = local.github_repo_id
  alert_email       = local.alert_email
  guardduty_enabled = false # owner ruling 2026-11-12: skip the monthly cost for this MVP

  depends_on = [terraform_data.literal_guard]
}

output "tfstate_bucket" {
  description = "State bucket; must equal the bucket in env/prod.*.backend.hcl."
  value       = module.account.tfstate_bucket
}

output "ci_plan_role_arn" {
  description = "-> GitHub repository variable AWS_PLAN_ROLE_ARN_PROD (only once a plan job exists)."
  value       = module.account.ci_plan_role_arn
}

output "ci_deploy_role_arn" {
  description = "-> GitHub environment 'prod' variable AWS_DEPLOY_ROLE_ARN."
  value       = module.account.ci_deploy_role_arn
}

output "lambda_boundary_arn" {
  description = "Permission boundary ARN for Lambda roles."
  value       = module.account.lambda_boundary_arn
}
