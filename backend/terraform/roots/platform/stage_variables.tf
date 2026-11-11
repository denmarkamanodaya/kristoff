# Stage inputs shared by both roots (values: env/<stage>.tfvars). This file is byte-identical in roots/platform and
# roots/compute (the CI step "roots share stage_variables.tf" fails on any drift). Validation lives in the stacks, the
# modules and the stage guard (one source per rule); these declarations only carry the values in.

variable "stage" {
  description = "Stage of the tfvars file: prod (validated by the stage guard and stage-names)."
  type        = string
  nullable    = false
}

variable "account_id" {
  description = "12-digit AWS account id of kristoff-<stage>; the stage guard compares it with the caller and the state bucket."
  type        = string
  nullable    = false
}

variable "region" {
  description = "AWS region of every resource of the stage (validated by stage-names: ap-southeast-2 only)."
  type        = string
  nullable    = false
}
