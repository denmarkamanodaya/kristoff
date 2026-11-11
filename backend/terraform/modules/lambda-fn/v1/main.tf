# lambda-fn v1 — one Lambda function with its execution role, inline policy, log group, published version,
# `live` alias, optional DLQ and optional asynchronous retry setting on the alias (stack/terraform.md: every function
# publishes and carries a `live` alias).
# Traps: the module never creates triggers (API routes, event-source mappings, invoke permissions): those belong to
# the stack. Routes and mappings must target `alias_invoke_arn` / `alias_arn`, never the unqualified function.

data "aws_iam_policy_document" "assume" {
  statement {
    sid     = "LambdaAssume"
    effect  = "Allow"
    actions = ["sts:AssumeRole"]
    principals {
      type        = "Service"
      identifiers = ["lambda.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "this" {
  name                 = var.name
  path                 = var.role_path
  description          = "Execution role of ${var.name}"
  assume_role_policy   = data.aws_iam_policy_document.assume.json
  permissions_boundary = var.permissions_boundary_arn
  tags                 = merge(var.tags, { Name = var.name })
}

resource "aws_cloudwatch_log_group" "this" {
  name              = "/aws/lambda/${var.name}"
  retention_in_days = var.log_retention_days
  tags              = merge(var.tags, { Name = "/aws/lambda/${var.name}" })
}

resource "aws_sqs_queue" "dlq" {
  count = var.dlq_enabled ? 1 : 0

  name                      = "${var.name}-dlq"
  message_retention_seconds = 1209600 # 14 days, the longest SQS allows
  sqs_managed_sse_enabled   = true
  tags                      = merge(var.tags, { Name = "${var.name}-dlq" })
}

# The action lists live in locals so `granted_actions` (outputs.tf) is known at plan time: the rendered JSON is not
# (it embeds the log group ARN).
locals {
  logs_actions = ["logs:CreateLogStream", "logs:PutLogEvents"]
  # X-Ray write actions only when tracing is Active: a PassThrough function never calls X-Ray (least privilege).
  xray_actions = var.tracing_mode == "Active" ? ["xray:PutTraceSegments", "xray:PutTelemetryRecords"] : []
  dlq_actions  = var.dlq_enabled ? ["sqs:SendMessage"] : []
  caller_actions = flatten([
    for s in jsondecode(var.policy_json).Statement : try(tolist(s.Action), [s.Action]) if s.Effect == "Allow"
  ])
}

data "aws_iam_policy_document" "baseline" {
  statement {
    sid       = "Logs"
    effect    = "Allow"
    actions   = local.logs_actions
    resources = ["${aws_cloudwatch_log_group.this.arn}:*"]
  }

  # X-Ray has no resource-level permissions (Service Authorization Reference), hence "*".
  dynamic "statement" {
    for_each = length(local.xray_actions) > 0 ? [1] : []
    content {
      sid       = "XRay"
      effect    = "Allow"
      actions   = local.xray_actions
      resources = ["*"]
    }
  }

  dynamic "statement" {
    for_each = aws_sqs_queue.dlq
    content {
      sid       = "DeadLetterQueue"
      effect    = "Allow"
      actions   = local.dlq_actions
      resources = [statement.value.arn]
    }
  }
}

data "aws_iam_policy_document" "role" {
  source_policy_documents = [data.aws_iam_policy_document.baseline.json, var.policy_json]
}

resource "aws_iam_role_policy" "this" {
  name   = "function"
  role   = aws_iam_role.this.id
  policy = data.aws_iam_policy_document.role.json
}

resource "aws_lambda_function" "this" {
  function_name    = var.name
  description      = var.description
  role             = aws_iam_role.this.arn
  handler          = var.handler
  runtime          = var.runtime
  architectures    = [var.architecture]
  filename         = var.zip_path
  source_code_hash = filebase64sha256(var.zip_path)
  memory_size      = var.memory_mb
  timeout          = var.timeout_s
  publish          = true
  # -1 is the provider's "unreserved" sentinel; a new account (quota 10) cannot host a reservation.
  reserved_concurrent_executions = var.reserved_concurrency_enabled ? var.reserved_concurrency : -1

  dynamic "environment" {
    for_each = length(var.env) > 0 ? [var.env] : []
    content {
      variables = environment.value
    }
  }

  dynamic "dead_letter_config" {
    for_each = aws_sqs_queue.dlq
    content {
      target_arn = dead_letter_config.value.arn
    }
  }

  tracing_config {
    mode = var.tracing_mode
  }

  logging_config {
    # Text: the backend's redacting logger already emits one JSON object per line.
    log_format = "Text"
    log_group  = aws_cloudwatch_log_group.this.name
  }

  tags = merge(var.tags, { Name = var.name })

  depends_on = [aws_iam_role_policy.this]
}

resource "aws_lambda_alias" "live" {
  name             = "live"
  description      = "Version the routes and mappings target; rollback = point it at the previous version"
  function_name    = aws_lambda_function.this.function_name
  function_version = aws_lambda_function.this.version
}

# Asynchronous invocations of the alias (schedules): how often Lambda retries a failed run. Unmanaged (Lambda's default
# of 2 retries) unless set; a scheduled job that is not safe to rerun at once sets 0 and waits for its next run.
resource "aws_lambda_function_event_invoke_config" "live" {
  count = var.async_retry_attempts == null ? 0 : 1

  function_name          = aws_lambda_function.this.function_name
  qualifier              = aws_lambda_alias.live.name
  maximum_retry_attempts = var.async_retry_attempts
}
