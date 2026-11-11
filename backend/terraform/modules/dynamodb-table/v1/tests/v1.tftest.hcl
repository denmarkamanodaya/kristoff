# dynamodb-table v1 — plan-mode tests, no AWS calls.

provider "aws" {
  region                      = "ap-southeast-2"
  access_key                  = "test"
  secret_key                  = "test"
  skip_credentials_validation = true
  skip_requesting_account_id  = true
  skip_metadata_api_check     = true
  skip_region_validation      = true
}

variables {
  name      = "kristoff-prod"
  hash_key  = "PK"
  range_key = "SK"
  attribute_types = {
    PK     = "S"
    SK     = "S"
    GSI1PK = "S"
    GSI1SK = "S"
  }
  global_secondary_indexes = [
    {
      name            = "GSI1"
      hash_key        = "GSI1PK"
      range_key       = "GSI1SK"
      projection_type = "ALL"
    },
  ]
}

run "table_is_protected_on_demand_with_ttl" {
  command = plan

  assert {
    condition     = aws_dynamodb_table.this.billing_mode == "PAY_PER_REQUEST"
    error_message = "tables are on-demand"
  }
  assert {
    condition     = aws_dynamodb_table.this.deletion_protection_enabled == true && aws_dynamodb_table.this.point_in_time_recovery[0].enabled == true
    error_message = "deletion protection and PITR are always on"
  }
  assert {
    condition     = aws_dynamodb_table.this.ttl[0].attribute_name == "ttl" && aws_dynamodb_table.this.ttl[0].enabled == true
    error_message = "TTL defaults to the `ttl` attribute"
  }
  assert {
    condition     = length(aws_dynamodb_table.this.attribute) == 4
    error_message = "all four key attributes (table and index keys) are declared"
  }
  assert {
    condition     = aws_dynamodb_table.this.server_side_encryption[0].enabled == true
    error_message = "server-side encryption is enabled"
  }
  assert {
    condition     = output.index_names == tolist(["GSI1"])
    error_message = "index_names lists the GSIs"
  }
}

run "rejects_undeclared_key_attribute" {
  command = plan

  variables {
    attribute_types = {
      PK = "S"
      SK = "S"
    }
  }

  expect_failures = [aws_dynamodb_table.this]
}

run "rejects_bad_attribute_type" {
  command = plan

  variables {
    attribute_types = {
      PK     = "S"
      SK     = "X"
      GSI1PK = "S"
      GSI1SK = "S"
    }
  }

  expect_failures = [var.attribute_types]
}
