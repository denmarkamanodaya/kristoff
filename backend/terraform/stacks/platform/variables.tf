# platform stack — inputs. Values come from env/<stage>.tfvars and env/<stage>.platform.tfvars through tf.sh.

variable "stage" {
  description = "Deployment stage: prod (validated by stage-names)."
  type        = string
  nullable    = false
}

variable "account_id" {
  description = "12-digit AWS account id of kristoff-<stage>; names embed it. The stage guard in the root compares it with the caller."
  type        = string
  nullable    = false
}

variable "region" {
  description = "AWS region of the stage; validated by stage-names (ap-southeast-2 only)."
  type        = string
  nullable    = false
  default     = "ap-southeast-2"
}

variable "alert_email" {
  description = "Owner mailbox subscribed to the alerts topic (the owner confirms the SNS mail once). A TODO_* marker fails validation on purpose."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^[^@[:space:]]+@[^@[:space:]]+\\.[^@[:space:]]+$", var.alert_email))
    error_message = "alert_email must be an e-mail address (env/<stage>.platform.tfvars)."
  }
}
