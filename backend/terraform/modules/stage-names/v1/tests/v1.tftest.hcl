# stage-names v1 — plan-mode tests. No provider, no AWS calls. Expected names are the ones docs/contract.md states.

variables {
  stage      = "prod"
  account_id = "123456789012"
}

run "names_follow_the_contract" {
  command = plan

  assert {
    condition     = output.prefix == "kristoff-prod" && output.table_name == "kristoff-prod-core"
    error_message = "prefix is kristoff-<stage>, the table is kristoff-prod-core (contract Storage)"
  }
  assert {
    condition     = output.docs_bucket_name == "kristoff-prod-docs-123456789012" && length(output.docs_bucket_name) <= 63
    error_message = "the docs bucket embeds the account id and fits 63 characters"
  }
  assert {
    condition     = output.docs_objects_arn == "arn:aws:s3:::kristoff-prod-docs-123456789012/docs/*"
    error_message = "PDF objects live under docs/ (contract key docs/<uid>/<docId>.pdf)"
  }
  assert {
    condition     = output.boundary_arn == "arn:aws:iam::123456789012:policy/kristoff/ci/kristoff-prod-lambda-boundary"
    error_message = "boundary ARN is built from the CI path and the boundary name"
  }
  assert {
    condition     = output.tfstate_bucket_name == "kristoff-tfstate-123456789012" && output.tfstate_key_prefix == "kristoff/prod"
    error_message = "state bucket and key prefix"
  }
}

run "ssm_names_match_the_contract" {
  command = plan

  assert {
    condition     = jsonencode(output.secret_param_names) == jsonencode(["/kristoff/prod/anthropic-api-key", "/kristoff/prod/totp-secret", "/kristoff/prod/session-hmac-key"]) && output.ai_enabled_param_name == "/kristoff/prod/ai-enabled"
    error_message = "the four SSM names are the contract's"
  }
  assert {
    condition     = output.ai_enabled_param_arn == "arn:aws:ssm:ap-southeast-2:123456789012:parameter/kristoff/prod/ai-enabled"
    error_message = "the ARN drops the leading slash of the name into parameter/..."
  }
  assert {
    condition     = alltrue([for a in concat(output.secret_param_arns, [output.ai_enabled_param_arn]) : startswith(a, trimsuffix(output.ssm_parameter_arn_pattern, "*"))])
    error_message = "every parameter sits inside the stage pattern the Lambda boundary admits"
  }
}

run "lambda_data_action_ceilings" {
  command = plan

  assert {
    condition     = contains(output.lambda_s3_bucket_actions, "s3:ListBucketVersions") && !contains(output.lambda_s3_object_actions, "s3:ListBucketVersions")
    error_message = "ListBucketVersions is a bucket-level action, never granted on objects"
  }
  assert {
    condition     = output.lambda_dlq_arn_pattern == "arn:aws:sqs:ap-southeast-2:123456789012:kristoff-prod-*-dlq"
    error_message = "DLQs are <prefix>-<key>-dlq"
  }
}

run "todo_marker_passes_validation" {
  command = plan

  variables {
    account_id = "TODO_PROD_ACCOUNT_ID"
  }

  assert {
    condition     = output.tfstate_bucket_name == "kristoff-tfstate-TODO_PROD_ACCOUNT_ID"
    error_message = "a TODO marker flows through so roots validate offline"
  }
}

run "rejects_foreign_region" {
  command = plan

  variables {
    region = "us-east-1"
  }

  expect_failures = [var.region]
}

run "rejects_dev_stage" {
  command = plan

  variables {
    stage = "dev"
  }

  expect_failures = [var.stage]
}

run "rejects_bad_account_id" {
  command = plan

  variables {
    account_id = "123"
  }

  expect_failures = [var.account_id]
}
