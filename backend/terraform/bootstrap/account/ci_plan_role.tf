# ci-plan: the read-only, data-blind role PR workflows may assume (pull_request OIDC subject).
# Traps: plans run with -lock=false, so the role must never hold PutObject on the state bucket; the explicit Denies
# keep it blind to the owner's data even if an Allow below is ever widened by mistake. No workflow uses it yet (the PR
# workflow is offline-only); it exists so a read-only plan job can be added without a bootstrap change.

resource "aws_iam_role" "ci_plan" {
  name                 = "${local.name_prefix}-ci-plan"
  path                 = local.ci_role_path
  description          = "GitHub Actions pull_request plans for ${local.name_prefix}: read-only, data-blind"
  assume_role_policy   = data.aws_iam_policy_document.ci_plan_trust.json
  max_session_duration = 3600
  tags                 = { Name = "${local.name_prefix}-ci-plan" }
}

data "aws_iam_policy_document" "ci_plan" {
  statement {
    sid    = "ReadScopedResources"
    effect = "Allow"
    actions = [
      "lambda:GetFunction", "lambda:GetFunctionConfiguration", "lambda:GetFunctionCodeSigningConfig",
      "lambda:GetFunctionConcurrency", "lambda:GetFunctionEventInvokeConfig", "lambda:GetFunctionUrlConfig",
      "lambda:GetFunctionRecursionConfig", "lambda:GetRuntimeManagementConfig",
      "lambda:ListVersionsByFunction", "lambda:GetAlias", "lambda:ListAliases", "lambda:GetPolicy", "lambda:ListTags",
      "dynamodb:DescribeTable", "dynamodb:DescribeContinuousBackups", "dynamodb:DescribeTimeToLive",
      "dynamodb:DescribeTableReplicaAutoScaling", "dynamodb:DescribeKinesisStreamingDestination",
      "dynamodb:ListTagsOfResource",
      "s3:ListBucket", "s3:GetBucketLocation", "s3:GetBucketPolicy", "s3:GetBucketAcl", "s3:GetBucketCORS",
      "s3:GetBucketWebsite", "s3:GetBucketVersioning", "s3:GetAccelerateConfiguration", "s3:GetBucketRequestPayment",
      "s3:GetBucketLogging", "s3:GetLifecycleConfiguration", "s3:GetReplicationConfiguration",
      "s3:GetEncryptionConfiguration", "s3:GetBucketObjectLockConfiguration", "s3:GetBucketTagging",
      "s3:GetBucketOwnershipControls", "s3:GetBucketPublicAccessBlock",
      "logs:ListTagsForResource",
      "cloudwatch:ListTagsForResource",
      "sns:GetTopicAttributes", "sns:GetSubscriptionAttributes", "sns:ListSubscriptionsByTopic", "sns:ListTagsForResource",
      "amplify:GetApp", "amplify:GetBranch", "amplify:ListTagsForResource",
      "iam:GetRole", "iam:GetRolePolicy", "iam:ListRolePolicies", "iam:ListAttachedRolePolicies",
      "iam:ListRoleTags", "iam:ListInstanceProfilesForRole", "iam:GetPolicy", "iam:GetPolicyVersion",
      "iam:ListPolicyVersions", "iam:ListPolicyTags",
      "ssm:ListTagsForResource",
    ]
    resources = concat(
      [
        "${module.names.lambda_function_arn_prefix}*",
        module.names.table_arn, "${module.names.table_arn}/index/*",
        module.names.docs_bucket_arn,
        aws_s3_bucket.tfstate.arn,
        "${local.log_group_arn_pattern}:*",
        "arn:aws:cloudwatch:${var.region}:${var.account_id}:alarm:${local.name_prefix}-*",
        "arn:aws:sns:${var.region}:${var.account_id}:${local.name_prefix}-*",
        "arn:aws:iam::${var.account_id}:role${local.lambda_role_path}*",
        "arn:aws:iam::${var.account_id}:policy${local.ci_role_path}*",
        module.names.ai_enabled_param_arn,
      ],
      local.amplify_app_arns,
      local.amplify_child_arns,
    )
  }

  statement {
    sid       = "ReadState"
    effect    = "Allow"
    actions   = ["s3:GetObject"]
    resources = ["${aws_s3_bucket.tfstate.arn}/${module.names.tfstate_key_prefix}/*"]
  }

  # Describe/list calls without resource-level permissions (AWS service reference), hence "*".
  statement {
    sid    = "ReadUnscopedApis"
    effect = "Allow"
    actions = [
      "lambda:ListFunctions", "lambda:GetAccountSettings",
      "apigateway:GET",
      "logs:DescribeLogGroups", "cloudwatch:DescribeAlarms",
      "ssm:DescribeParameters", "sts:GetCallerIdentity", "iam:ListAccountAliases",
    ]
    resources = ["*"]
  }

  statement {
    sid       = "SimulateCiRoles"
    effect    = "Allow"
    actions   = ["iam:SimulatePrincipalPolicy", "iam:GetContextKeysForPrincipalPolicy"]
    resources = ["arn:aws:iam::${var.account_id}:role${local.ci_role_path}*"]
  }

  # A plan refreshes the kill switch with GetParameter; reading any OTHER parameter (the SecureStrings) is denied.
  statement {
    sid       = "ReadKillSwitch"
    effect    = "Allow"
    actions   = ["ssm:GetParameter"]
    resources = [module.names.ai_enabled_param_arn]
  }

  statement {
    sid    = "DenyOwnerDataAndWrites"
    effect = "Deny"
    actions = [
      "dynamodb:GetItem", "dynamodb:BatchGetItem", "dynamodb:Query", "dynamodb:Scan",
      "dynamodb:PutItem", "dynamodb:UpdateItem", "dynamodb:DeleteItem", "dynamodb:BatchWriteItem",
      "ssm:PutParameter", "ssm:DeleteParameter*",
      "sts:AssumeRole", "iam:CreateUser", "iam:CreateAccessKey", "iam:CreateLoginProfile",
      "lambda:InvokeFunction", "lambda:InvokeFunctionUrl", "lambda:UpdateFunctionCode", "lambda:UpdateFunctionConfiguration",
      "amplify:CreateDeployment", "amplify:StartDeployment",
      "bedrock:InvokeModel*",
    ]
    resources = ["*"]
  }

  statement {
    sid           = "DenySsmReadsExceptKillSwitch"
    effect        = "Deny"
    actions       = ["ssm:GetParameter*"]
    not_resources = [module.names.ai_enabled_param_arn]
  }

  # Objects in the docs bucket are never readable by a PR; the state bucket is readable but never writable
  # (no lock object either: plans run with -lock=false).
  statement {
    sid       = "DenyDocObjects"
    effect    = "Deny"
    actions   = ["s3:GetObject*", "s3:PutObject*", "s3:DeleteObject*"]
    resources = ["${module.names.docs_bucket_arn}/*"]
  }

  statement {
    sid       = "DenyStateWrites"
    effect    = "Deny"
    actions   = ["s3:PutObject*", "s3:DeleteObject*"]
    resources = ["${aws_s3_bucket.tfstate.arn}/*"]
  }
}

resource "aws_iam_policy" "ci_plan" {
  name        = "${local.name_prefix}-ci-plan"
  path        = local.ci_role_path
  description = "Read-only, data-blind plan permissions for ${aws_iam_role.ci_plan.name}"
  policy      = data.aws_iam_policy_document.ci_plan.json
  tags        = { Name = "${local.name_prefix}-ci-plan" }

  lifecycle {
    precondition {
      condition     = length(data.aws_iam_policy_document.ci_plan.minified_json) <= 6144
      error_message = "ci-plan policy exceeds the 6,144-character managed-policy limit; split it."
    }
  }
}

resource "aws_iam_role_policy_attachment" "ci_plan" {
  role       = aws_iam_role.ci_plan.name
  policy_arn = aws_iam_policy.ci_plan.arn
}
