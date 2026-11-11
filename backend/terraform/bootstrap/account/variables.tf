# Bootstrap account module — inputs. Every literal comes from bootstrap/<stage>/main.tf.
# Trap: the owner replaces the TODO_* markers there; the root's preconditions fail the plan until then.

variable "stage" {
  description = "Deployment stage this account hosts: prod. Names every resource (kristoff-<stage>-*)."
  type        = string
  nullable    = false
  validation {
    condition     = contains(["prod"], var.stage)
    error_message = "stage must be \"prod\"."
  }
}

variable "account_id" {
  description = "The 12-digit id of the AWS member account this module bootstraps (a TODO_* marker passes validation so the roots validate offline; the root's plan-time precondition refuses it)."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^([0-9]{12}|TODO_[A-Z_]+)$", var.account_id))
    error_message = "account_id must be the 12-digit account id; replace the TODO marker in bootstrap/<stage>/main.tf."
  }
}

variable "region" {
  description = "Home region (ap-southeast-2), the only region the kristoff-region-allowlist SCP allows."
  type        = string
  nullable    = false
  default     = "ap-southeast-2"
}

variable "github_owner" {
  description = "GitHub login that owns the repository (part of the immutable OIDC subject)."
  type        = string
  nullable    = false
}

variable "github_owner_id" {
  description = "Numeric GitHub user id of the owner (gh api users/<login> --jq .id); immutable across renames."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^[0-9]+$", var.github_owner_id))
    error_message = "github_owner_id must be numeric."
  }
}

variable "github_repo" {
  description = "Repository name (without owner) that CI runs from."
  type        = string
  nullable    = false
}

variable "github_repo_id" {
  description = "Numeric GitHub repository id (gh api repos/<owner>/<repo> --jq .id, available once the repo exists); immutable across renames."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^([0-9]+|TODO_[A-Z_]+)$", var.github_repo_id))
    error_message = "github_repo_id must be numeric; replace the TODO marker in bootstrap/<stage>/main.tf."
  }
}

variable "alert_email" {
  description = "Owner mailbox for the AWS Budget notifications (80 % actual, 80 % forecast, 100 % actual)."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^([^@[:space:]]+@[^@[:space:]]+\\.[^@[:space:]]+|TODO_[A-Z_]+)$", var.alert_email))
    error_message = "alert_email must be an e-mail address; replace the TODO marker in bootstrap/<stage>/main.tf."
  }
}

variable "guardduty_enabled" {
  description = "Create a GuardDuty detector."
  type        = bool
  nullable    = false
  default     = false
}

variable "budget_limit_usd" {
  description = "Monthly AWS cost budget for the account in USD (plan: 20). It covers AWS only; Anthropic API spend is billed separately (set a monthly limit in the Anthropic Console)."
  type        = string
  nullable    = false
  default     = "20"
  validation {
    condition     = can(regex("^[0-9]+(\\.[0-9]+)?$", var.budget_limit_usd))
    error_message = "budget_limit_usd must be a decimal number."
  }
}

variable "sso_permission_set_name" {
  description = "IAM Identity Center permission set whose provisioned role may read and write the state objects."
  type        = string
  nullable    = false
  default     = "KristoffAdmin"
}
