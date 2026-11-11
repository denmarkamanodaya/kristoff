# lambda-fn v1 — plan-mode tests, no AWS calls (dummy credentials, validation skipped).

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
  name                     = "kristoff-prod-health"
  zip_path                 = "./tests/fixtures/empty.zip"
  role_path                = "/kristoff/lambda/"
  permissions_boundary_arn = "arn:aws:iam::123456789012:policy/kristoff/ci/kristoff-prod-lambda-boundary"
  policy_json              = "{\"Version\":\"2012-10-17\",\"Statement\":[{\"Sid\":\"Table\",\"Effect\":\"Allow\",\"Action\":[\"dynamodb:DescribeTable\"],\"Resource\":\"arn:aws:dynamodb:ap-southeast-2:123456789012:table/kristoff-prod\"}]}"
  env                      = { STAGE = "prod", LOG_LEVEL = "info" }
}

run "defaults" {
  command = plan

  assert {
    condition     = aws_lambda_function.this.function_name == "kristoff-prod-health"
    error_message = "function name must equal var.name"
  }
  assert {
    condition     = aws_lambda_function.this.runtime == "nodejs24.x" && aws_lambda_function.this.architectures[0] == "arm64"
    error_message = "runtime defaults to nodejs24.x on arm64"
  }
  assert {
    condition     = aws_lambda_function.this.publish == true && aws_lambda_alias.live.name == "live"
    error_message = "every function publishes a version and carries the live alias"
  }
  assert {
    condition     = aws_lambda_function.this.reserved_concurrent_executions == -1
    error_message = "reserved concurrency stays unset while reserved_concurrency_enabled is false"
  }
  assert {
    condition     = aws_iam_role.this.path == "/kristoff/lambda/" && aws_iam_role.this.permissions_boundary == var.permissions_boundary_arn
    error_message = "role must live under the Lambda path with the permission boundary"
  }
  assert {
    condition     = aws_cloudwatch_log_group.this.retention_in_days == 14 && aws_cloudwatch_log_group.this.name == "/aws/lambda/kristoff-prod-health"
    error_message = "log group is /aws/lambda/<name> with 14-day retention"
  }
  assert {
    condition     = length(aws_sqs_queue.dlq) == 0 && output.dlq_arn == null
    error_message = "no DLQ unless dlq_enabled"
  }
  assert {
    condition     = output.granted_actions == tolist(["dynamodb:DescribeTable", "logs:CreateLogStream", "logs:PutLogEvents"])
    error_message = "granted_actions is the sorted union of the module statements and policy_json"
  }
  assert {
    condition     = length(aws_lambda_function_event_invoke_config.live) == 0
    error_message = "the asynchronous retry setting stays unmanaged unless async_retry_attempts is set"
  }
  assert {
    condition     = output.env_keys == tolist(["LOG_LEVEL", "STAGE"])
    error_message = "env_keys lists the env map keys sorted"
  }
  assert {
    condition     = length(data.aws_iam_policy_document.baseline.statement) == 1 && data.aws_iam_policy_document.baseline.statement[0].sid == "Logs"
    error_message = "with PassThrough tracing and no DLQ the module adds exactly the logs statement"
  }
}

run "reserved_concurrency_applies_when_enabled" {
  command = plan

  variables {
    reserved_concurrency_enabled = true
    reserved_concurrency         = 20
  }

  assert {
    condition     = aws_lambda_function.this.reserved_concurrent_executions == 20
    error_message = "reserved concurrency applies when enabled"
  }
}

run "async_retries_apply_to_the_live_alias_when_set" {
  command = plan

  variables {
    async_retry_attempts = 0
  }

  assert {
    condition     = aws_lambda_function_event_invoke_config.live[0].maximum_retry_attempts == 0 && aws_lambda_function_event_invoke_config.live[0].qualifier == "live"
    error_message = "async_retry_attempts sets the retry count on the live alias"
  }
}

run "async_retries_reject_out_of_range" {
  command = plan

  variables {
    async_retry_attempts = 3
  }

  expect_failures = [var.async_retry_attempts]
}

run "dlq_creates_queue_and_grants_send" {
  command = plan

  variables {
    dlq_enabled = true
  }

  assert {
    condition     = aws_sqs_queue.dlq[0].name == "kristoff-prod-health-dlq" && aws_sqs_queue.dlq[0].sqs_managed_sse_enabled == true
    error_message = "DLQ is <name>-dlq with SSE-SQS"
  }
  assert {
    condition     = contains(output.granted_actions, "sqs:SendMessage")
    error_message = "the role may send to its DLQ"
  }
}

run "xray_actions_only_when_tracing_is_active" {
  command = plan

  variables {
    tracing_mode = "Active"
  }

  assert {
    condition     = contains(output.granted_actions, "xray:PutTraceSegments") && contains(output.granted_actions, "xray:PutTelemetryRecords")
    error_message = "Active tracing needs the two X-Ray write actions"
  }
}

run "rejects_unknown_runtime" {
  command = plan

  variables {
    runtime = "python3.12"
  }

  expect_failures = [var.runtime]
}

run "rejects_policy_without_statements" {
  command = plan

  variables {
    policy_json = "{\"Version\":\"2012-10-17\"}"
  }

  expect_failures = [var.policy_json]
}
