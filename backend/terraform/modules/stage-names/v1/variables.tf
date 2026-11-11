# stage-names v1 — inputs. The stage and the account come from env/<stage>.tfvars (or the bootstrap root literals).

variable "stage" {
  description = "Deployment stage: prod (Kristoff has one stage)."
  type        = string
  nullable    = false
  validation {
    condition     = contains(["prod"], var.stage)
    error_message = "stage must be \"prod\"."
  }
}

variable "account_id" {
  description = "12-digit AWS account id of kristoff-<stage>. A TODO_* marker passes validation so roots validate offline; the roots' preconditions refuse it at plan time."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^([0-9]{12}|TODO_[A-Z_]+)$", var.account_id))
    error_message = "account_id must be 12 digits (or a TODO_* marker)."
  }
}

variable "region" {
  description = "AWS region of the stage (the region allow-list SCP admits ap-southeast-2 only)."
  type        = string
  nullable    = false
  default     = "ap-southeast-2"
  validation {
    condition     = var.region == "ap-southeast-2"
    error_message = "region must be ap-southeast-2 (bootstrap/scp/kristoff-region-allowlist.json)."
  }
}
