# dynamodb-table v1 — one on-demand DynamoDB table with optional GSIs and TTL, PITR and deletion protection ALWAYS on
# (stack/terraform.md: tables get both, and prevent_destroy).
# Traps: a GSI projection cannot change after creation (changing it replaces the index); the plan guard blocks
# delete/replace of the table and any PITR or deletion-protection flip; SSE stays the DynamoDB default (AWS-owned key,
# no charge).

locals {
  # Every key attribute (table and indexes) must be declared; a non-key attribute here is an error in DynamoDB.
  key_attributes = toset(concat(
    [var.hash_key],
    var.range_key == null ? [] : [var.range_key],
    flatten([for g in var.global_secondary_indexes : concat([g.hash_key], g.range_key == null ? [] : [g.range_key])]),
  ))
}

resource "aws_dynamodb_table" "this" {
  name                        = var.name
  billing_mode                = "PAY_PER_REQUEST"
  hash_key                    = var.hash_key
  range_key                   = var.range_key
  deletion_protection_enabled = true

  dynamic "attribute" {
    for_each = { for k in local.key_attributes : k => var.attribute_types[k] }
    content {
      name = attribute.key
      type = attribute.value
    }
  }

  dynamic "global_secondary_index" {
    for_each = { for g in var.global_secondary_indexes : g.name => g }
    content {
      name               = global_secondary_index.key
      projection_type    = global_secondary_index.value.projection_type
      non_key_attributes = global_secondary_index.value.projection_type == "INCLUDE" ? global_secondary_index.value.non_key_attributes : null

      # key_schema replaces the deprecated hash_key/range_key arguments of an index (provider >= 6.67 warns on them).
      key_schema {
        attribute_name = global_secondary_index.value.hash_key
        key_type       = "HASH"
      }

      dynamic "key_schema" {
        for_each = global_secondary_index.value.range_key == null ? [] : [global_secondary_index.value.range_key]
        content {
          attribute_name = key_schema.value
          key_type       = "RANGE"
        }
      }
    }
  }

  dynamic "ttl" {
    for_each = var.ttl_attribute == null ? [] : [var.ttl_attribute]
    content {
      attribute_name = ttl.value
      enabled        = true
    }
  }

  point_in_time_recovery {
    enabled = true
  }

  server_side_encryption {
    enabled = true
  }

  tags = merge(var.tags, { Name = var.name })

  lifecycle {
    prevent_destroy = true

    precondition {
      condition     = alltrue([for k in local.key_attributes : contains(keys(var.attribute_types), k)])
      error_message = "attribute_types must declare the type of every key attribute (table and index keys)."
    }
  }
}
