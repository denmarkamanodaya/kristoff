# lambda-stream-url v1 — inputs. Function inputs are passed through to lambda-fn v1 (same meaning, same validation
# there); only the URL's CORS origins are new. Every value comes from the calling stack.

variable "name" {
  description = "Fully prefixed function name, e.g. kristoff-prod-ask-stream; also the role name and the log group suffix."
  type        = string
  nullable    = false
}

variable "description" {
  description = "Human description shown in the Lambda console."
  type        = string
  nullable    = false
  default     = ""
}

variable "zip_path" {
  description = "Path to the deployment bundle (backend/dist/<function key>.zip, or the stack's placeholder bundle)."
  type        = string
  nullable    = false
}

variable "handler" {
  description = "Handler export, <file>.<function> of the ESM bundle."
  type        = string
  nullable    = false
  default     = "index.handler"
}

variable "runtime" {
  description = "Lambda runtime identifier (validated by lambda-fn v1)."
  type        = string
  nullable    = false
  default     = "nodejs24.x"
}

variable "architecture" {
  description = "Instruction set architecture (validated by lambda-fn v1)."
  type        = string
  nullable    = false
  default     = "arm64"
}

variable "memory_mb" {
  description = "Memory in MB (validated by lambda-fn v1)."
  type        = number
  nullable    = false
  default     = 512
}

variable "timeout_s" {
  description = "Timeout in seconds (1-900); a function URL has no 30 s gateway cut-off, so a stream may run up to this."
  type        = number
  nullable    = false
  default     = 300
}

variable "env" {
  description = "Environment variables (names and non-secret values only; secrets are read at runtime by name)."
  type        = map(string)
  nullable    = false
  default     = {}
}

variable "role_path" {
  description = "IAM path of the execution role (validated by lambda-fn v1)."
  type        = string
  nullable    = false
}

variable "permissions_boundary_arn" {
  description = "ARN of the permission boundary policy the role must carry (validated by lambda-fn v1)."
  type        = string
  nullable    = false
}

variable "policy_json" {
  description = "IAM policy document (JSON) with the function's own statements; lambda-fn v1 adds the logs statement."
  type        = string
  nullable    = false
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days (validated by lambda-fn v1)."
  type        = number
  nullable    = false
  default     = 30
}

variable "reserved_concurrency_enabled" {
  description = "Apply `reserved_concurrency` (stays false while the account quota is the new-account default)."
  type        = bool
  nullable    = false
  default     = false
}

variable "reserved_concurrency" {
  description = "Reserved concurrent executions applied when `reserved_concurrency_enabled` is true."
  type        = number
  nullable    = false
  default     = 5
}

variable "cors_allowed_origins" {
  description = "Exact browser origins the URL's CORS allows (https://host[:port], or http://localhost:<port>); never *. Methods are POST, headers authorization and content-type, Retry-After exposed."
  type        = list(string)
  nullable    = false
  validation {
    condition     = length(var.cors_allowed_origins) > 0 && alltrue([for o in var.cors_allowed_origins : can(regex("^https://[a-z0-9.-]+(:[0-9]+)?$", o)) || can(regex("^http://localhost:[0-9]+$", o))])
    error_message = "cors_allowed_origins must list at least one exact origin; never * (stack/security.md)."
  }
}

variable "tags" {
  description = "Extra tags; the provider's default_tags carry Project/Stage/Stack/ManagedBy/Repo, lambda-fn adds Name."
  type        = map(string)
  nullable    = false
  default     = {}
}
