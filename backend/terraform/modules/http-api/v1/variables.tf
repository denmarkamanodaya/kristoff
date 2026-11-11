# http-api v1 — inputs. Routes are data (route key -> alias invoke ARN); auth is "lambda" (the REQUEST authorizer) or
# "none". Trap: a "lambda" route needs `lambda_authorizer_enabled = true` (validated).

variable "name" {
  description = "API name, e.g. kristoff-prod-api."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^[a-z0-9-]{3,64}$", var.name))
    error_message = "name must be 3-64 chars of [a-z0-9-]."
  }
}

variable "description" {
  description = "Human description of the API."
  type        = string
  nullable    = false
  default     = ""
}

variable "routes" {
  description = "Routes to create: route_key (\"GET /v1/config\"), target_invoke_arn (the Lambda `live` alias invoke ARN), auth (\"lambda\" | \"none\"), optional per-route throttles."
  type = list(object({
    route_key              = string
    target_invoke_arn      = string
    auth                   = string
    throttling_rate_limit  = optional(number)
    throttling_burst_limit = optional(number)
  }))
  nullable = false
  validation {
    condition     = alltrue([for r in var.routes : contains(["lambda", "none"], r.auth)])
    error_message = "every route's auth must be \"lambda\" or \"none\"."
  }
  validation {
    condition     = alltrue([for r in var.routes : can(regex("^(GET|POST|PUT|PATCH|DELETE) /", r.route_key))])
    error_message = "route_key must be \"<METHOD> /path\" (no ANY: CORS preflight must stay with API Gateway)."
  }
  validation {
    condition     = length(var.routes) == length(distinct([for r in var.routes : r.route_key]))
    error_message = "route keys must be unique."
  }
  validation {
    condition     = var.lambda_authorizer_enabled || alltrue([for r in var.routes : r.auth == "none"])
    error_message = "a \"lambda\" route requires lambda_authorizer_enabled = true."
  }
}

variable "throttling_rate_limit" {
  description = "Stage default steady-state requests per second."
  type        = number
  nullable    = false
  validation {
    condition     = var.throttling_rate_limit > 0
    error_message = "throttling_rate_limit must be positive."
  }
}

variable "throttling_burst_limit" {
  description = "Stage default burst."
  type        = number
  nullable    = false
  validation {
    condition     = var.throttling_burst_limit > 0
    error_message = "throttling_burst_limit must be positive."
  }
}

variable "lambda_authorizer_enabled" {
  description = "Create the Lambda REQUEST authorizer (simple responses, no cache, identity source the Authorization header)."
  type        = bool
  nullable    = false
  default     = true
}

variable "authorizer_invoke_arn" {
  description = "Invoke ARN of the authorizer function's `live` alias; required when the authorizer is enabled."
  type        = string
  nullable    = false
  default     = ""
  validation {
    condition     = !var.lambda_authorizer_enabled || can(regex("^arn:aws:apigateway:", var.authorizer_invoke_arn))
    error_message = "authorizer_invoke_arn must be a Lambda invoke ARN (arn:aws:apigateway:...) when the authorizer is enabled."
  }
}

variable "cors_allowed_origins" {
  description = "Exact browser origins allowed by CORS (https://host[:port], or http://localhost:<port>); never *."
  type        = list(string)
  nullable    = false
  validation {
    condition     = length(var.cors_allowed_origins) > 0 && alltrue([for o in var.cors_allowed_origins : can(regex("^https://[a-z0-9.-]+(:[0-9]+)?$", o)) || can(regex("^http://localhost:[0-9]+$", o))])
    error_message = "cors_allowed_origins must list at least one exact origin; never * (stack/security.md)."
  }
}

variable "cors_allowed_methods" {
  description = "HTTP methods CORS allows."
  type        = list(string)
  nullable    = false
  default     = ["GET", "POST", "PATCH"]
}

variable "cors_allowed_headers" {
  description = "Request headers CORS allows."
  type        = list(string)
  nullable    = false
  default     = ["authorization", "content-type"]
}

variable "access_log_group_name" {
  description = "CloudWatch log group for access logs, e.g. /kristoff/prod/api-access."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^/[a-zA-Z0-9/_.-]+$", var.access_log_group_name))
    error_message = "access_log_group_name must be a log-group path starting with '/'."
  }
}

variable "access_log_retention_days" {
  description = "Access-log retention in days."
  type        = number
  nullable    = false
  default     = 30
}

variable "access_log_format" {
  description = "Access-log line format; the default carries no path+query, no headers and no bodies (questions and document titles are personal data)."
  type        = string
  nullable    = false
  default     = "{\"requestId\":\"$context.requestId\",\"ip\":\"$context.identity.sourceIp\",\"route\":\"$context.routeKey\",\"status\":\"$context.status\",\"latency\":\"$context.responseLatency\",\"authorizerError\":\"$context.authorizer.error\",\"err\":\"$context.integrationErrorMessage\"}"
}

variable "tags" {
  description = "Extra tags; the module adds Name."
  type        = map(string)
  nullable    = false
  default     = {}
}
