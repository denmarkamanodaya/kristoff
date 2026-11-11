# compute stack — inputs. Values come from env/<stage>.tfvars and env/<stage>.compute.tfvars through tf.sh. A default
# here is a contract fact (docs/contract.md) and stays out of the tfvars unless the contract changes.

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

variable "web_origin" {
  description = "The one web origin (platform output web_origin, https://main.<Amplify default domain>): CORS of the HTTP API and the stream URL, and WEB_ORIGINS of the functions. Validated by the modules (exact origin, never *)."
  type        = string
  nullable    = false
}

variable "lambda_zip_dir" {
  description = "Directory holding <function key>.zip bundles (backend/dist). A key without a bundle deploys the placeholder bundle, which answers 503."
  type        = string
  nullable    = false
}

variable "lambda_scope_path" {
  description = "Path of backend/lambda-scope.json (per-function env keys and data actions, written by the backend build). A missing file fails the plan."
  type        = string
  nullable    = false
}

variable "owner_id" {
  description = "OWNER_ID: the constant principal id every storage key is scoped by (contract \"Principal\")."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^[a-z0-9-]{1,64}$", var.owner_id))
    error_message = "owner_id must be 1-64 chars of [a-z0-9-]."
  }
}

variable "model_id" {
  description = "MODEL_ID: Anthropic model id the ask function calls (stage data)."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^claude-[a-z0-9-]+$", var.model_id))
    error_message = "model_id must be a claude-* model id."
  }
}

variable "max_tokens" {
  description = "MAX_TOKENS: max output tokens per answer (stage data)."
  type        = number
  nullable    = false
  validation {
    condition     = var.max_tokens >= 256 && var.max_tokens <= 64000 && floor(var.max_tokens) == var.max_tokens
    error_message = "max_tokens must be an integer between 256 and 64000."
  }
}

variable "daily_question_cap" {
  description = "DAILY_QUESTION_CAP: questions per UTC day before DAILY_CAP_REACHED (stage data)."
  type        = number
  nullable    = false
  validation {
    condition     = var.daily_question_cap >= 1 && var.daily_question_cap <= 10000 && floor(var.daily_question_cap) == var.daily_question_cap
    error_message = "daily_question_cap must be an integer between 1 and 10000 (backend env schema)."
  }
}

variable "anthropic_timeout_ms" {
  description = "ANTHROPIC_TIMEOUT_MS: timeout of one model call; must end before the stream function's own timeout."
  type        = number
  nullable    = false
  validation {
    condition     = var.anthropic_timeout_ms >= 1000 && var.anthropic_timeout_ms <= 140000 && floor(var.anthropic_timeout_ms) == var.anthropic_timeout_ms
    error_message = "anthropic_timeout_ms must be an integer between 1000 and 140000 (backend env schema)."
  }
}

variable "max_attached_docs" {
  description = "MAX_ATTACHED_DOCS (contract: 5)."
  type        = number
  nullable    = false
  default     = 5
}

variable "max_history_turns" {
  description = "MAX_HISTORY_TURNS (contract: 20)."
  type        = number
  nullable    = false
  default     = 20
}

variable "session_ttl_seconds" {
  description = "SESSION_TTL_SECONDS: session token lifetime (contract: 43200)."
  type        = number
  nullable    = false
  default     = 43200
}

variable "http_memory_mb" {
  description = "Memory of every HTTP function and the authorizer (contract: 256 MB)."
  type        = number
  nullable    = false
  default     = 256
}

variable "http_timeout_s" {
  description = "Timeout of every HTTP function and the authorizer (contract: 10 s)."
  type        = number
  nullable    = false
  default     = 10
}

variable "stream_memory_mb" {
  description = "Memory of ask-stream (contract: 512 MB)."
  type        = number
  nullable    = false
  default     = 512
}

variable "stream_timeout_s" {
  description = "Timeout of ask-stream (contract: 300 s)."
  type        = number
  nullable    = false
  default     = 300
}

variable "api_throttling_rate_limit" {
  description = "HTTP API $default stage steady rate, requests per second (contract: 10)."
  type        = number
  nullable    = false
  default     = 10
}

variable "api_throttling_burst_limit" {
  description = "HTTP API $default stage burst (contract: 20)."
  type        = number
  nullable    = false
  default     = 20
}

variable "log_retention_days" {
  description = "Retention of every function log group in days."
  type        = number
  nullable    = false
  default     = 30
}
