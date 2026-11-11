# Permission boundary: the ceiling for every Lambda execution role CI creates.
# Traps: every action a Lambda role needs must be INSIDE this set (the per-function roles the compute stack derives from
# backend/lambda-scope.json are subsets, and its scope_guard refuses anything outside stage-names lambda_*_actions, the
# same lists used here); a boundary is a ceiling, not a grant. A NEW boundary statement needs an owner-run
# `terraform apply` in bootstrap/<stage> BEFORE the deploy that relies on it.

data "aws_iam_policy_document" "lambda_boundary" {
  statement {
    sid       = "DynamoDbItems"
    effect    = "Allow"
    actions   = module.names.lambda_dynamodb_actions
    resources = [module.names.table_arn, "${module.names.table_arn}/index/*"]
  }

  statement {
    sid       = "S3DocObjects"
    effect    = "Allow"
    actions   = module.names.lambda_s3_object_actions
    resources = [module.names.docs_objects_arn]
  }

  # Listing object versions (docs-delete removes every version of a doc) is a bucket-level action; it is admitted only
  # for keys under docs/. The function role narrows it further to docs/<OWNER_ID>/* (the boundary has no owner id).
  statement {
    sid       = "S3DocVersionsList"
    effect    = "Allow"
    actions   = module.names.lambda_s3_bucket_actions
    resources = [module.names.docs_bucket_arn]
    condition {
      test     = "StringLike"
      variable = "s3:prefix"
      values   = ["${module.names.docs_object_prefix}*"]
    }
  }

  # docs-confirm starts docs-split asynchronously; a function may invoke only functions of this stage (the role names
  # the one alias).
  statement {
    sid       = "InvokeStageFunctions"
    effect    = "Allow"
    actions   = ["lambda:InvokeFunction"]
    resources = ["${module.names.lambda_function_arn_prefix}*"]
  }

  # Failed asynchronous invocations go to the function's own dead-letter queue (lambda-fn dlq_enabled).
  statement {
    sid       = "SendToDeadLetterQueues"
    effect    = "Allow"
    actions   = ["sqs:SendMessage"]
    resources = [module.names.lambda_dlq_arn_pattern]
  }

  # The kill switch and the three owner-set SecureStrings.
  statement {
    sid       = "SsmReadStageParameters"
    effect    = "Allow"
    actions   = ["ssm:GetParameter"]
    resources = [module.names.ssm_parameter_arn_pattern]
  }

  # Decrypting a SecureString under the AWS-managed aws/ssm key. Its key policy (read 2026-11-12 in kristoff-prod) allows
  # kms:Decrypt to Principal "*" with kms:CallerAccount + kms:ViaService = ssm, so the function roles need no grant of
  # their own. The boundary still has to admit the action: an implicit deny in a permissions boundary limits what a
  # resource policy grants to a role SESSION, which is what a "*" principal matches. Scoped to calls made through SSM
  # for this stage's parameters (SSM sets the encryption context PARAMETER_ARN); the managed key's id is random, hence
  # key/* in this account.
  statement {
    sid       = "KmsDecryptStageParametersViaSsm"
    effect    = "Allow"
    actions   = ["kms:Decrypt"]
    resources = ["arn:aws:kms:${var.region}:${var.account_id}:key/*"]
    condition {
      test     = "StringEquals"
      variable = "kms:ViaService"
      values   = ["ssm.${var.region}.amazonaws.com"]
    }
    condition {
      test     = "StringLike"
      variable = "kms:EncryptionContext:PARAMETER_ARN"
      values   = [module.names.ssm_parameter_arn_pattern]
    }
  }

  statement {
    sid       = "Logs"
    effect    = "Allow"
    actions   = ["logs:CreateLogStream", "logs:PutLogEvents"]
    resources = ["${local.log_group_arn_pattern}:/aws/lambda/${local.name_prefix}-*"]
  }

  # X-Ray has no resource-level permissions (Service Authorization Reference).
  statement {
    sid       = "XRay"
    effect    = "Allow"
    actions   = ["xray:PutTraceSegments", "xray:PutTelemetryRecords"]
    resources = ["*"]
  }
}

resource "aws_iam_policy" "lambda_boundary" {
  name        = module.names.boundary_name
  path        = local.ci_role_path
  description = "Permission boundary for every ${local.name_prefix} Lambda execution role"
  policy      = data.aws_iam_policy_document.lambda_boundary.json
  tags        = { Name = module.names.boundary_name }

  lifecycle {
    precondition {
      condition     = length(data.aws_iam_policy_document.lambda_boundary.minified_json) <= 6144
      error_message = "lambda boundary exceeds the 6,144-character managed-policy limit."
    }
  }
}
