# stage-guard v1 — inputs. stage and account_id come from env/<stage>.tfvars; stack is a literal in the calling root;
# backend_state_json is the file `terraform init` wrote, read by the root (never typed by a human).

variable "stack" {
  description = "Stack of the calling root (roots/<stack>): platform or compute; part of the expected state key and every error message."
  type        = string
  nullable    = false
  validation {
    condition     = contains(["platform", "compute"], var.stack)
    error_message = "stack must be platform or compute."
  }
}

variable "stage" {
  description = "Stage the tfvars claim: prod (Kristoff has one stage)."
  type        = string
  nullable    = false
  validation {
    condition     = contains(["prod"], var.stage)
    error_message = "stage must be prod."
  }
}

variable "account_id" {
  description = "12-digit AWS account id of kristoff-<stage> from env/<stage>.tfvars; the state bucket name embeds it. Not validated here on purpose: a malformed value must fail the plan with the guard's message, not a variable error."
  type        = string
  nullable    = false
}

variable "backend_state_json" {
  description = "Contents of <TF_DATA_DIR>/terraform.tfstate, the backend record `terraform init` writes (type + config: bucket, key, use_lockfile, encrypt), or \"\" when absent. The root reads it with try(file(...), \"\"); an empty or foreign value fails the plan."
  type        = string
  nullable    = false
}
