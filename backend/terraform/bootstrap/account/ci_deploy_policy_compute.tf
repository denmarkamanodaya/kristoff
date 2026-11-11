# ci-deploy policy "compute": Lambda (functions, aliases, function URLs), logs, alarms, SNS.
# Trap: Terraform reads concurrency, policy, aliases, URL config and invoke config on every plan; `publish = true` plus
# the `live` alias need the version and alias actions. Every action name is in the AWS service reference for its service.

data "aws_iam_policy_document" "ci_deploy_compute" {
  statement {
    sid    = "Lambda"
    effect = "Allow"
    actions = [
      "lambda:CreateFunction", "lambda:UpdateFunctionCode", "lambda:UpdateFunctionConfiguration", "lambda:DeleteFunction",
      "lambda:GetFunction", "lambda:GetFunctionConfiguration", "lambda:GetFunctionCodeSigningConfig",
      "lambda:GetFunctionConcurrency", "lambda:GetFunctionEventInvokeConfig", "lambda:GetFunctionRecursionConfig",
      "lambda:GetRuntimeManagementConfig", "lambda:ListVersionsByFunction", "lambda:PublishVersion",
      "lambda:CreateAlias", "lambda:UpdateAlias", "lambda:DeleteAlias", "lambda:GetAlias", "lambda:ListAliases",
      "lambda:PutFunctionConcurrency", "lambda:DeleteFunctionConcurrency",
      "lambda:CreateFunctionUrlConfig", "lambda:UpdateFunctionUrlConfig", "lambda:DeleteFunctionUrlConfig",
      "lambda:GetFunctionUrlConfig",
      "lambda:AddPermission", "lambda:RemovePermission", "lambda:GetPolicy",
      "lambda:TagResource", "lambda:UntagResource", "lambda:ListTags",
    ]
    resources = ["${module.names.lambda_function_arn_prefix}*"]
  }

  # No resource-level permissions for these (Service Authorization Reference).
  statement {
    sid       = "LambdaAccount"
    effect    = "Allow"
    actions   = ["lambda:ListFunctions", "lambda:GetAccountSettings"]
    resources = ["*"]
  }

  statement {
    sid    = "Logs"
    effect = "Allow"
    actions = [
      "logs:CreateLogGroup", "logs:DeleteLogGroup", "logs:PutRetentionPolicy", "logs:DeleteRetentionPolicy",
      "logs:ListTagsForResource", "logs:TagResource", "logs:UntagResource",
    ]
    resources = [
      "${local.log_group_arn_pattern}:/aws/lambda/${local.name_prefix}-*",
      "${local.log_group_arn_pattern}:${module.names.log_group_prefix}/*",
    ]
  }

  statement {
    sid       = "LogsDescribe"
    effect    = "Allow"
    actions   = ["logs:DescribeLogGroups"]
    resources = ["*"]
  }

  # API Gateway access logs are CloudWatch "vended logs": enabling them on a stage needs the log-delivery actions, which
  # support no resource-level permissions (dexfs prior art: "Insufficient permissions to enable logging" otherwise).
  statement {
    sid    = "LogsVendedDelivery"
    effect = "Allow"
    actions = [
      "logs:CreateLogDelivery", "logs:GetLogDelivery", "logs:UpdateLogDelivery", "logs:DeleteLogDelivery",
      "logs:ListLogDeliveries", "logs:PutResourcePolicy", "logs:DescribeResourcePolicies",
    ]
    resources = ["*"]
  }

  statement {
    sid       = "Alarms"
    effect    = "Allow"
    actions   = ["cloudwatch:PutMetricAlarm", "cloudwatch:DeleteAlarms", "cloudwatch:TagResource", "cloudwatch:UntagResource", "cloudwatch:ListTagsForResource"]
    resources = ["arn:aws:cloudwatch:${var.region}:${var.account_id}:alarm:${local.name_prefix}-*"]
  }

  statement {
    sid       = "AlarmsDescribe"
    effect    = "Allow"
    actions   = ["cloudwatch:DescribeAlarms"]
    resources = ["*"]
  }

  statement {
    sid    = "Sns"
    effect = "Allow"
    actions = [
      "sns:CreateTopic", "sns:DeleteTopic", "sns:GetTopicAttributes", "sns:SetTopicAttributes",
      "sns:GetSubscriptionAttributes", "sns:SetSubscriptionAttributes", "sns:Subscribe", "sns:Unsubscribe",
      "sns:ListSubscriptionsByTopic", "sns:ListTagsForResource", "sns:TagResource", "sns:UntagResource",
    ]
    resources = ["arn:aws:sns:${var.region}:${var.account_id}:${local.name_prefix}-*"]
  }
}
