# ci-deploy policy "deny": the hard line, evaluated before any Allow. Denies may use action wildcards: they only ever
# narrow what the role can do.
# Trap: the plan guard (scripts/tf-plan-guard.jq) is not the only line — a deleted table or bucket, a read secret, a
# self-modified CI role or a read of the owner's data is denied here even if every other control fails.

data "aws_iam_policy_document" "ci_deploy_deny" {
  statement {
    sid    = "DenyDestructiveAndAccountControls"
    effect = "Deny"
    actions = [
      "dynamodb:DeleteTable",
      "s3:DeleteBucket",
      "kms:ScheduleKeyDeletion", "kms:DisableKey",
      "iam:CreateUser", "iam:CreateAccessKey", "iam:CreateLoginProfile", "iam:DeleteRolePermissionsBoundary",
      "cloudtrail:*", "guardduty:*", "organizations:*", "account:*", "budgets:*",
      "sts:AssumeRole",
    ]
    resources = ["*"]
  }

  # SSM stays unreadable and unwritable for CI EXCEPT the kill switch (SsmKillSwitch in ci_deploy_policy_data.tf).
  # NotResource makes "every other parameter" the denied set, so the owner-set SecureStrings (Anthropic key, TOTP secret,
  # session key) stay out of reach.
  statement {
    sid           = "DenySsmExceptKillSwitch"
    effect        = "Deny"
    actions       = ["ssm:GetParameter*", "ssm:PutParameter", "ssm:DeleteParameter*"]
    not_resources = local.ci_ssm_config_param_arns
  }

  # Never the owner's data: item-level DynamoDB, invoking a function (an invoke could return data).
  statement {
    sid    = "DenyOwnerData"
    effect = "Deny"
    actions = [
      "dynamodb:GetItem", "dynamodb:BatchGetItem", "dynamodb:Query", "dynamodb:Scan",
      "dynamodb:PutItem", "dynamodb:UpdateItem", "dynamodb:DeleteItem", "dynamodb:BatchWriteItem",
      "dynamodb:PartiQLSelect", "dynamodb:PartiQLInsert", "dynamodb:PartiQLUpdate", "dynamodb:PartiQLDelete",
      "lambda:InvokeFunction", "lambda:InvokeFunctionUrl",
      "bedrock:InvokeModel*",
    ]
    resources = ["*"]
  }

  statement {
    sid       = "DenyDocObjects"
    effect    = "Deny"
    actions   = ["s3:GetObject*", "s3:PutObject*", "s3:DeleteObject*"]
    resources = ["${module.names.docs_bucket_arn}/*"]
  }

  statement {
    sid     = "DenyCiRoleAndPolicyChanges"
    effect  = "Deny"
    actions = ["iam:*"]
    resources = [
      "arn:aws:iam::${var.account_id}:role${local.ci_role_path}*",
      "arn:aws:iam::${var.account_id}:policy${local.ci_role_path}*",
    ]
  }

  statement {
    sid       = "DenyAttachingManagedPolicies"
    effect    = "Deny"
    actions   = ["iam:AttachRolePolicy"]
    resources = ["*"]
  }
}
