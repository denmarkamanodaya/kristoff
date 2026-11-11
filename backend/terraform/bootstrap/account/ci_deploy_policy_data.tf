# ci-deploy policy "data": DynamoDB, S3 (docs bucket + state), API Gateway v2, Amplify Hosting, the kill-switch parameter,
# read-only APIs.
# Traps: no item-level DynamoDB action at all; bucket-level S3 only on the docs bucket (object reads there are an explicit
# Deny in the deny policy: a "/*" grant would let a deploy read the owner's PDFs); the native lockfile needs Put/Delete
# on <key>.tflock. Every action name is in the AWS service reference for its service.

data "aws_iam_policy_document" "ci_deploy_data" {
  statement {
    sid    = "DynamoDb"
    effect = "Allow"
    actions = [
      "dynamodb:CreateTable", "dynamodb:UpdateTable", "dynamodb:DescribeTable",
      "dynamodb:DescribeContinuousBackups", "dynamodb:UpdateContinuousBackups",
      "dynamodb:DescribeTimeToLive", "dynamodb:UpdateTimeToLive",
      "dynamodb:TagResource", "dynamodb:UntagResource", "dynamodb:ListTagsOfResource",
      "dynamodb:DescribeTableReplicaAutoScaling", "dynamodb:DescribeKinesisStreamingDestination",
    ]
    resources = [module.names.table_arn, "${module.names.table_arn}/index/*"]
  }

  # The bucket-level calls the s3-bucket module makes on create and on every refresh (aws_s3_bucket reads ACL, CORS,
  # website, versioning, accelerate, request payment, logging, lifecycle, replication, encryption, object lock, tags,
  # policy).
  statement {
    sid    = "S3DocsBucket"
    effect = "Allow"
    actions = [
      "s3:CreateBucket", "s3:ListBucket", "s3:GetBucketLocation",
      "s3:GetBucketPolicy", "s3:PutBucketPolicy", "s3:DeleteBucketPolicy",
      "s3:GetBucketAcl", "s3:GetBucketCORS", "s3:PutBucketCORS", "s3:GetBucketWebsite",
      "s3:GetBucketVersioning", "s3:PutBucketVersioning", "s3:GetAccelerateConfiguration",
      "s3:GetBucketRequestPayment", "s3:GetBucketLogging",
      "s3:GetLifecycleConfiguration", "s3:PutLifecycleConfiguration", "s3:GetReplicationConfiguration",
      "s3:GetEncryptionConfiguration", "s3:PutEncryptionConfiguration", "s3:GetBucketObjectLockConfiguration",
      "s3:GetBucketTagging", "s3:PutBucketTagging",
      "s3:GetBucketOwnershipControls", "s3:PutBucketOwnershipControls",
      "s3:GetBucketPublicAccessBlock", "s3:PutBucketPublicAccessBlock",
    ]
    resources = [module.names.docs_bucket_arn]
  }

  statement {
    sid       = "S3StateBucket"
    effect    = "Allow"
    actions   = ["s3:ListBucket", "s3:GetBucketVersioning", "s3:GetBucketLocation"]
    resources = [aws_s3_bucket.tfstate.arn]
  }

  statement {
    sid       = "S3StateObjects"
    effect    = "Allow"
    actions   = ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"]
    resources = ["${aws_s3_bucket.tfstate.arn}/${module.names.tfstate_key_prefix}/*"]
  }

  # API Gateway ids are random; the account is the scope. Tags are written with POST/PUT/DELETE on /tags/*, and
  # CreateStage with tags is authorised as apigateway:TagResource (dexfs first deploy, 2026-10-19: AccessDenied on
  # apigateway:TagResource for /apis/<id>/stages). The service reference does not list TagResource/UntagResource, but
  # IAM evaluates them, so both stay.
  statement {
    sid     = "ApiGatewayV2"
    effect  = "Allow"
    actions = ["apigateway:GET", "apigateway:POST", "apigateway:PUT", "apigateway:PATCH", "apigateway:DELETE", "apigateway:TagResource", "apigateway:UntagResource"]
    resources = [
      "arn:aws:apigateway:${var.region}::/apis",
      "arn:aws:apigateway:${var.region}::/apis/*",
      "arn:aws:apigateway:${var.region}::/tags/*",
    ]
  }

  # Amplify Hosting: the app and its branch (platform stack; no custom domain) and the zip deployments
  # (deploy-prod.yml: CreateDeployment, StartDeployment, GetJob). CreateApp and ListApps have no resource type in the
  # service reference, hence "*".
  statement {
    sid       = "AmplifyCreate"
    effect    = "Allow"
    actions   = ["amplify:CreateApp", "amplify:ListApps"]
    resources = ["*"]
  }

  statement {
    sid    = "AmplifyManage"
    effect = "Allow"
    actions = [
      "amplify:GetApp", "amplify:UpdateApp", "amplify:DeleteApp",
      "amplify:CreateBranch", "amplify:GetBranch", "amplify:UpdateBranch", "amplify:DeleteBranch", "amplify:ListBranches",
      "amplify:CreateDeployment", "amplify:StartDeployment", "amplify:GetJob", "amplify:ListJobs",
      "amplify:TagResource", "amplify:UntagResource", "amplify:ListTagsForResource",
    ]
    resources = concat(local.amplify_app_arns, local.amplify_child_arns)
  }

  # The kill switch the platform stack creates (String, not secret). GetParameter is needed because the provider reads
  # the value back on every refresh; the deny policy keeps every OTHER parameter unreadable (NotResource).
  # DescribeParameters has no resource type, so it sits in "ReadOnly" below.
  statement {
    sid    = "SsmKillSwitch"
    effect = "Allow"
    actions = [
      "ssm:PutParameter", "ssm:GetParameter", "ssm:DeleteParameter",
      "ssm:AddTagsToResource", "ssm:RemoveTagsFromResource", "ssm:ListTagsForResource",
    ]
    resources = local.ci_ssm_config_param_arns
  }

  statement {
    sid       = "ReadOnly"
    effect    = "Allow"
    actions   = ["sts:GetCallerIdentity", "iam:ListAccountAliases", "ssm:DescribeParameters"]
    resources = ["*"]
  }
}
