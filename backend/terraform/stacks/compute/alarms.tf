# Alarms -> the alerts topic the platform stack creates (addressed by its deterministic ARN). Thresholds are for one
# owner's traffic: any real failure is worth a mail. treat_missing_data = notBreaching: no traffic is not a failure.

resource "aws_cloudwatch_metric_alarm" "api_5xx_rate" {
  alarm_name          = "${module.names.prefix}-api-5xx-rate"
  alarm_description   = "More than 5% of API requests returned 5xx in 15 minutes (at least 5 requests). Read the api-access log group, then the failing function's log group."
  evaluation_periods  = 1
  threshold           = 0.05
  comparison_operator = "GreaterThanThreshold"
  treat_missing_data  = "notBreaching"
  alarm_actions       = [module.names.alerts_topic_arn]
  ok_actions          = [module.names.alerts_topic_arn]

  metric_query {
    id          = "rate"
    expression  = "IF(requests >= 5, errors / requests, 0)"
    label       = "API 5xx rate"
    return_data = true
  }

  metric_query {
    id = "errors"
    metric {
      namespace   = "AWS/ApiGateway"
      metric_name = "5xx"
      dimensions  = { ApiId = module.api.api_id }
      period      = 900
      stat        = "Sum"
    }
  }

  metric_query {
    id = "requests"
    metric {
      namespace   = "AWS/ApiGateway"
      metric_name = "Count"
      dimensions  = { ApiId = module.api.api_id }
      period      = 900
      stat        = "Sum"
    }
  }
}

resource "aws_cloudwatch_metric_alarm" "ask_stream_errors" {
  alarm_name          = "${module.names.prefix}-ask-stream-errors"
  alarm_description   = "ask-stream failed (uncaught error or timeout) at least once in 5 minutes. Read its log group (prompts and answers are never logged)."
  namespace           = "AWS/Lambda"
  metric_name         = "Errors"
  statistic           = "Sum"
  period              = 300
  evaluation_periods  = 1
  threshold           = 1
  comparison_operator = "GreaterThanOrEqualToThreshold"
  treat_missing_data  = "notBreaching"
  dimensions          = { FunctionName = module.ask.function_name }
  alarm_actions       = [module.names.alerts_topic_arn]
  ok_actions          = [module.names.alerts_topic_arn]
}

# Account-level Lambda throttles (no dimension = every function in the account; the account hosts only Kristoff). A new
# account's concurrency quota is 10, so a throttle means the quota or a loop, not load.
resource "aws_cloudwatch_metric_alarm" "lambda_throttles" {
  alarm_name          = "${module.names.prefix}-lambda-throttles"
  alarm_description   = "Lambda throttled at least one invocation in 5 minutes. Check the account concurrency quota (Service Quotas) and for a client retry loop."
  namespace           = "AWS/Lambda"
  metric_name         = "Throttles"
  statistic           = "Sum"
  period              = 300
  evaluation_periods  = 1
  threshold           = 1
  comparison_operator = "GreaterThanOrEqualToThreshold"
  treat_missing_data  = "notBreaching"
  alarm_actions       = [module.names.alerts_topic_arn]
  ok_actions          = [module.names.alerts_topic_arn]
}

# docs-split runs asynchronously (contract v2 "Large PDFs"); an invocation that still fails after its retry lands in its
# dead-letter queue, and the doc stays "processing". Any message there needs a human (stack/errors-and-resilience.md).
resource "aws_cloudwatch_metric_alarm" "docs_split_dlq" {
  alarm_name          = "${module.names.prefix}-docs-split-dlq"
  alarm_description   = "A docs-split invocation failed twice (crash or timeout) and is in its dead-letter queue (<prefix>-docs-split-dlq). Read the docs-split log group; the doc stays 'processing' until it is deleted or re-confirmed."
  namespace           = "AWS/SQS"
  metric_name         = "ApproximateNumberOfMessagesVisible"
  statistic           = "Maximum"
  period              = 300
  evaluation_periods  = 1
  threshold           = 0
  comparison_operator = "GreaterThanThreshold"
  treat_missing_data  = "notBreaching"
  dimensions          = { QueueName = element(split(":", module.fn["docs-split"].dlq_arn), 5) } # queue name = last ARN field
  alarm_actions       = [module.names.alerts_topic_arn]
  ok_actions          = [module.names.alerts_topic_arn]
}
