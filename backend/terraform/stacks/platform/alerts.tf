# Alerts topic kristoff-<stage>-alerts with one e-mail subscriber. CloudWatch alarms publish to it; Budgets mail the
# owner directly (bootstrap). No KMS key on purpose: CloudWatch alarms cannot publish to a topic encrypted with the
# AWS-managed aws/sns key, and a customer key costs US$1 a month per account for alarm names only.

#trivy:ignore:AVD-AWS-0095 payload is alarm names only; CloudWatch alarms cannot publish to a topic encrypted with the aws/sns key
resource "aws_sns_topic" "alerts" {
  name = module.names.alerts_topic_name
  tags = { Name = module.names.alerts_topic_name }
}

data "aws_iam_policy_document" "alerts_topic" {
  statement {
    sid       = "CloudWatchAlarmsPublish"
    effect    = "Allow"
    actions   = ["sns:Publish"]
    resources = [aws_sns_topic.alerts.arn]
    principals {
      type        = "Service"
      identifiers = ["cloudwatch.amazonaws.com"]
    }
    condition {
      test     = "StringEquals"
      variable = "aws:SourceAccount"
      values   = [var.account_id]
    }
  }
}

resource "aws_sns_topic_policy" "alerts" {
  arn    = aws_sns_topic.alerts.arn
  policy = data.aws_iam_policy_document.alerts_topic.json
}

resource "aws_sns_topic_subscription" "alerts_email" {
  topic_arn = aws_sns_topic.alerts.arn
  protocol  = "email"
  endpoint  = var.alert_email
}
