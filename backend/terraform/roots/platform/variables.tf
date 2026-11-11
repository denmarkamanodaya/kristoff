# roots/platform — platform-only inputs (values: env/<stage>.platform.tfvars). Shared stage inputs:
# stage_variables.tf. Validation stays in stacks/platform/variables.tf and the modules.

variable "alert_email" {
  description = "Owner mailbox subscribed to the alerts topic."
  type        = string
  nullable    = false
}
