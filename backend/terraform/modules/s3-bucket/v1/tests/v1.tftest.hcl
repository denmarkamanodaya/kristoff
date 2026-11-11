# s3-bucket v1 — plan-mode tests, no AWS calls.

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
  name              = "kristoff-prod-data-123456789012"
  expiring_prefixes = { "tmp/" = 2 }
}

run "private_versioned_encrypted" {
  command = plan

  assert {
    condition = alltrue([
      aws_s3_bucket_public_access_block.this.block_public_acls,
      aws_s3_bucket_public_access_block.this.block_public_policy,
      aws_s3_bucket_public_access_block.this.ignore_public_acls,
      aws_s3_bucket_public_access_block.this.restrict_public_buckets,
    ])
    error_message = "all four Block Public Access switches are on"
  }
  assert {
    condition     = aws_s3_bucket_versioning.this.versioning_configuration[0].status == "Enabled"
    error_message = "versioning is on"
  }
  assert {
    condition     = one(aws_s3_bucket_server_side_encryption_configuration.this.rule).apply_server_side_encryption_by_default[0].sse_algorithm == "AES256"
    error_message = "SSE-S3 is the default encryption"
  }
  assert {
    condition     = aws_s3_bucket_ownership_controls.this.rule[0].object_ownership == "BucketOwnerEnforced"
    error_message = "ACLs are disabled"
  }
  assert {
    condition     = strcontains(aws_s3_bucket_policy.this.policy, "aws:SecureTransport") && strcontains(aws_s3_bucket_policy.this.policy, "Deny")
    error_message = "the bucket policy denies non-TLS requests"
  }
}

run "lifecycle_expires_tmp_and_cleans_up" {
  command = plan

  assert {
    condition     = length(aws_s3_bucket_lifecycle_configuration.this.rule) == 3
    error_message = "one prefix rule + housekeeping + delete-marker rule"
  }
  assert {
    condition     = one([for r in aws_s3_bucket_lifecycle_configuration.this.rule : r if r.id == "expire-tmp"]).expiration[0].days == 2
    error_message = "tmp/ objects expire after 2 days"
  }
  assert {
    condition     = one([for r in aws_s3_bucket_lifecycle_configuration.this.rule : r if r.id == "housekeeping"]).noncurrent_version_expiration[0].noncurrent_days == 7
    error_message = "noncurrent versions expire after 7 days"
  }
}

run "no_prefix_rule_when_none_given" {
  command = plan

  variables {
    expiring_prefixes = {}
  }

  assert {
    condition     = length(aws_s3_bucket_lifecycle_configuration.this.rule) == 2
    error_message = "only housekeeping and delete-marker rules remain"
  }
}

run "rejects_prefix_without_trailing_slash" {
  command = plan

  variables {
    expiring_prefixes = { "tmp" = 2 }
  }

  expect_failures = [var.expiring_prefixes]
}

run "no_cors_unless_origins_are_given" {
  command = plan

  assert {
    condition     = length(aws_s3_bucket_cors_configuration.this) == 0
    error_message = "no CORS configuration without origins"
  }
}

run "cors_allows_only_the_listed_origins" {
  command = plan

  variables {
    cors_allowed_origins = ["https://kristoff.example.dev"]
  }

  assert {
    condition     = tolist(one(aws_s3_bucket_cors_configuration.this[0].cors_rule).allowed_origins) == tolist(["https://kristoff.example.dev"]) && toset(one(aws_s3_bucket_cors_configuration.this[0].cors_rule).allowed_methods) == toset(["GET", "PUT"])
    error_message = "CORS lists exactly the given origins with GET and PUT"
  }
}

run "rejects_wildcard_origin" {
  command = plan

  variables {
    cors_allowed_origins = ["*"]
  }

  expect_failures = [var.cors_allowed_origins]
}

run "rejects_uppercase_name" {
  command = plan

  variables {
    name = "DexFS-Data"
  }

  expect_failures = [var.name]
}
