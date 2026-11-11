# roots/compute — compute-only inputs (values: env/<stage>.compute.tfvars). Shared stage inputs: stage_variables.tf.
# Validation and the contract defaults stay in stacks/compute/variables.tf.

variable "owner_id" {
  description = "OWNER_ID of the functions (contract \"Principal\")."
  type        = string
  nullable    = false
}

variable "model_id" {
  description = "MODEL_ID: Anthropic model id."
  type        = string
  nullable    = false
}

variable "max_tokens" {
  description = "MAX_TOKENS: max output tokens per answer."
  type        = number
  nullable    = false
}

variable "daily_question_cap" {
  description = "DAILY_QUESTION_CAP: questions per UTC day."
  type        = number
  nullable    = false
}

variable "anthropic_timeout_ms" {
  description = "ANTHROPIC_TIMEOUT_MS: timeout of one model call."
  type        = number
  nullable    = false
}
