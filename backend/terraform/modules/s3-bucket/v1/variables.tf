# s3-bucket v1 — inputs. Every value comes from the calling stack; the module holds no stage or account literal.

variable "name" {
  description = "Globally unique bucket name, e.g. kristoff-prod-docs-<account id> (the account id makes it unique)."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]$", var.name))
    error_message = "name must be 3-63 chars of lowercase letters, digits, dots and hyphens."
  }
}

variable "expiring_prefixes" {
  description = "Prefix (ending in /) mapped to the number of days after which its objects expire, e.g. { \"tmp/\" = 2 } for unconfirmed uploads. Empty = nothing expires by prefix."
  type        = map(number)
  nullable    = false
  default     = {}
  validation {
    condition     = alltrue([for p, d in var.expiring_prefixes : endswith(p, "/") && d >= 1 && floor(d) == d])
    error_message = "every prefix must end with '/' and expire after a whole number of days >= 1."
  }
}

variable "noncurrent_version_expiration_days" {
  description = "Days after which a noncurrent object version is removed."
  type        = number
  nullable    = false
  default     = 7
  validation {
    condition     = var.noncurrent_version_expiration_days >= 1 && floor(var.noncurrent_version_expiration_days) == var.noncurrent_version_expiration_days
    error_message = "noncurrent_version_expiration_days must be a whole number >= 1."
  }
}

variable "abort_incomplete_multipart_days" {
  description = "Days after which an incomplete multipart upload is aborted."
  type        = number
  nullable    = false
  default     = 2
  validation {
    condition     = var.abort_incomplete_multipart_days >= 1 && floor(var.abort_incomplete_multipart_days) == var.abort_incomplete_multipart_days
    error_message = "abort_incomplete_multipart_days must be a whole number >= 1."
  }
}

variable "cors_allowed_origins" {
  description = "Exact browser origins (https://host, no path) allowed to call the bucket through presigned URLs; empty = no CORS configuration."
  type        = list(string)
  nullable    = false
  default     = []
  validation {
    condition     = alltrue([for o in var.cors_allowed_origins : can(regex("^https://[a-z0-9.-]+(:[0-9]+)?$", o)) || can(regex("^http://localhost:[0-9]+$", o))])
    error_message = "every origin must be https://<host>[:port] (or http://localhost:<port>); never * (stack/security.md)."
  }
}

variable "cors_allowed_methods" {
  description = "HTTP methods the CORS rule allows."
  type        = list(string)
  nullable    = false
  default     = ["GET", "PUT"]
  validation {
    condition     = length(var.cors_allowed_methods) > 0 && alltrue([for m in var.cors_allowed_methods : contains(["GET", "PUT", "HEAD"], m)])
    error_message = "cors_allowed_methods must be a non-empty subset of GET, PUT, HEAD."
  }
}

variable "tags" {
  description = "Extra tags; the provider's default_tags carry Project/Stage/Stack/ManagedBy/Repo, the module adds Name."
  type        = map(string)
  nullable    = false
  default     = {}
}
