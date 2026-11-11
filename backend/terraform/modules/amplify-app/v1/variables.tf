# amplify-app v1 — inputs. Every value comes from the calling stack; the module holds no stage or domain literal.

variable "name" {
  description = "Amplify app name, e.g. kristoff-prod-web."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^[a-z0-9-]{3,64}$", var.name))
    error_message = "name must be 3-64 chars of [a-z0-9-]."
  }
}

variable "description" {
  description = "Human description of the app."
  type        = string
  nullable    = false
  default     = ""
}

variable "branch_name" {
  description = "The one branch deployments go to."
  type        = string
  nullable    = false
  default     = "main"
}

variable "content_security_policy" {
  description = "Content-Security-Policy header value sent on every path (the stack builds it from the API, stream and bucket origins)."
  type        = string
  nullable    = false
  validation {
    condition     = strcontains(var.content_security_policy, "default-src") && !strcontains(var.content_security_policy, "unsafe-eval")
    error_message = "content_security_policy must set default-src and must not allow unsafe-eval."
  }
}

variable "tags" {
  description = "Extra tags; the provider's default_tags carry Project/Stage/Stack/ManagedBy/Repo, the module adds Name."
  type        = map(string)
  nullable    = false
  default     = {}
}
