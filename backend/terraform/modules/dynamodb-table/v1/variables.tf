# dynamodb-table v1 — inputs. Every value comes from the calling stack; the module holds no stage literal.

variable "name" {
  description = "Fully prefixed table name, e.g. kristoff-prod."
  type        = string
  nullable    = false
  validation {
    condition     = can(regex("^[a-zA-Z0-9_.-]{3,255}$", var.name))
    error_message = "name must be 3-255 chars of [a-zA-Z0-9_.-]."
  }
}

variable "hash_key" {
  description = "Partition key attribute name (e.g. PK)."
  type        = string
  nullable    = false
}

variable "range_key" {
  description = "Sort key attribute name (e.g. SK); null for a hash-only table."
  type        = string
  default     = null
}

variable "attribute_types" {
  description = "Type (S, N or B) of every KEY attribute of the table and its indexes, keyed by attribute name."
  type        = map(string)
  nullable    = false
  validation {
    condition     = alltrue([for t in values(var.attribute_types) : contains(["S", "N", "B"], t)])
    error_message = "attribute types must be S, N or B."
  }
}

variable "global_secondary_indexes" {
  description = "Global secondary indexes. The projection cannot change after creation; non_key_attributes only applies to INCLUDE."
  type = list(object({
    name               = string
    hash_key           = string
    range_key          = optional(string)
    projection_type    = string
    non_key_attributes = optional(list(string))
  }))
  nullable = false
  default  = []
  validation {
    condition     = alltrue([for g in var.global_secondary_indexes : contains(["ALL", "KEYS_ONLY", "INCLUDE"], g.projection_type)])
    error_message = "projection_type must be ALL, KEYS_ONLY or INCLUDE."
  }
  validation {
    condition     = length(var.global_secondary_indexes) == length(distinct([for g in var.global_secondary_indexes : g.name]))
    error_message = "index names must be unique."
  }
}

variable "ttl_attribute" {
  description = "Name of the epoch-seconds attribute DynamoDB expires items by; null disables TTL."
  type        = string
  default     = "ttl"
}

variable "tags" {
  description = "Extra tags; the provider's default_tags carry Project/Stage/Stack/ManagedBy/Repo, the module adds Name."
  type        = map(string)
  nullable    = false
  default     = {}
}
