# lambda-fn v1 — inputs. Every value comes from the calling stack; the module holds no stage or account literal.
# Trap: `policy_json` is the function's OWN statements; the module adds the logs statement, the DLQ statement (when
# enabled) and the X-Ray statement (only when tracing is Active) itself.

variable "name" {
  description = "Fully prefixed function name, e.g. kristoff-prod-health; also the role name and the log group suffix."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^[a-z0-9-]{3,64}$", var.name))
    error_message = "name must be 3-64 chars of [a-z0-9-]."
  }
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
  description = "Lambda runtime identifier."
  type        = string
  nullable    = false
  default     = "nodejs24.x"
  validation {
    condition     = contains(["nodejs22.x", "nodejs24.x"], var.runtime)
    error_message = "runtime must be nodejs22.x or nodejs24.x."
  }
}

variable "architecture" {
  description = "Instruction set architecture."
  type        = string
  nullable    = false
  default     = "arm64"
  validation {
    condition     = contains(["arm64", "x86_64"], var.architecture)
    error_message = "architecture must be arm64 or x86_64."
  }
}

variable "memory_mb" {
  description = "Memory in MB (128-10240); CPU scales with it."
  type        = number
  nullable    = false
  default     = 256
  validation {
    condition     = var.memory_mb >= 128 && var.memory_mb <= 10240 && floor(var.memory_mb) == var.memory_mb
    error_message = "memory_mb must be an integer between 128 and 10240."
  }
}

variable "timeout_s" {
  description = "Timeout in seconds (1-900); HTTP functions use 29 so the handler answers before API Gateway's 30 s cut-off."
  type        = number
  nullable    = false
  default     = 5
  validation {
    condition     = var.timeout_s >= 1 && var.timeout_s <= 900 && floor(var.timeout_s) == var.timeout_s
    error_message = "timeout_s must be an integer between 1 and 900."
  }
}

variable "env" {
  description = "Environment variables of the function (names and non-secret values only; secrets are read at runtime by name)."
  type        = map(string)
  nullable    = false
  default     = {}
}

variable "role_path" {
  description = "IAM path of the execution role; the deploy role may only manage roles under the Lambda path."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^/([a-z0-9-]+/)+$", var.role_path))
    error_message = "role_path must start and end with '/' (e.g. /kristoff/lambda/)."
  }
}

variable "permissions_boundary_arn" {
  description = "ARN of the permission boundary policy every Lambda role must carry (kristoff-<stage>-lambda-boundary)."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^arn:aws:iam::[0-9]{12}:policy/.+$", var.permissions_boundary_arn))
    error_message = "permissions_boundary_arn must be an IAM policy ARN."
  }
}

variable "policy_json" {
  description = "IAM policy document (JSON) with the function's own statements; merged with the module's logs (and X-Ray/DLQ when enabled) statements into one inline role policy."
  type        = string
  nullable    = false
  validation {
    condition     = can(jsondecode(var.policy_json).Statement)
    error_message = "policy_json must be a JSON policy document with a Statement array."
  }
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days."
  type        = number
  nullable    = false
  default     = 14
  validation {
    condition     = contains([1, 3, 5, 7, 14, 30, 60, 90, 120, 150, 180, 365, 400, 545, 731, 1096, 1827, 2192, 2557, 2922, 3288, 3653], var.log_retention_days)
    error_message = "log_retention_days must be one of the CloudWatch Logs retention values."
  }
}

variable "reserved_concurrency_enabled" {
  description = "Apply `reserved_concurrency`; stays false while the account quota is the new-account default of 10 (AWS keeps 100 unreserved)."
  type        = bool
  nullable    = false
  default     = false
}

variable "reserved_concurrency" {
  description = "Reserved concurrent executions applied when `reserved_concurrency_enabled` is true."
  type        = number
  nullable    = false
  default     = 20
  validation {
    condition     = var.reserved_concurrency >= 1 && floor(var.reserved_concurrency) == var.reserved_concurrency
    error_message = "reserved_concurrency must be a positive integer."
  }
}

variable "async_retry_attempts" {
  description = "Retries Lambda makes of a failed asynchronous invocation of the `live` alias (0-2); null leaves Lambda's default (2) unmanaged. Scheduled jobs set 0: the next scheduled run is the retry."
  type        = number
  nullable    = true
  default     = null
  validation {
    condition     = var.async_retry_attempts == null || contains([0, 1, 2], var.async_retry_attempts)
    error_message = "async_retry_attempts must be null, 0, 1 or 2."
  }
}

variable "dlq_enabled" {
  description = "Create an SQS dead-letter queue <name>-dlq for failed asynchronous invocations and grant sqs:SendMessage on it."
  type        = bool
  nullable    = false
  default     = false
}

variable "tracing_mode" {
  description = "X-Ray tracing mode."
  type        = string
  nullable    = false
  default     = "PassThrough"
  validation {
    condition     = contains(["PassThrough", "Active"], var.tracing_mode)
    error_message = "tracing_mode must be PassThrough or Active."
  }
}

variable "tags" {
  description = "Extra tags; the provider's default_tags carry Project/Stage/Stack/ManagedBy/Repo, the module adds Name."
  type        = map(string)
  nullable    = false
  default     = {}
}
