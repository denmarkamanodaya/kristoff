# bootstrap/account — plan-mode tests of the security-relevant shape, no AWS calls. Policy JSON that embeds a resource
# ARN created in the same plan is unknown here, so the tests read the data sources' statement blocks instead.

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
  stage             = "prod"
  account_id        = "123456789012"
  github_owner      = "denmarkamanodaya"
  github_owner_id   = "5304296"
  github_repo       = "kristoff"
  github_repo_id    = "1454246629"
  alert_email       = "owner@example.com"
  guardduty_enabled = true
}

run "ci_roles_trust_only_the_immutable_subjects_of_this_repo" {
  command = plan

  assert {
    condition     = toset(one([for c in data.aws_iam_policy_document.ci_deploy_trust.statement[0].condition : c if c.variable == "token.actions.githubusercontent.com:sub"]).values) == toset(["repo:denmarkamanodaya@5304296/kristoff@1454246629:environment:prod"])
    error_message = "ci-deploy trusts exactly the environment:prod subject in the immutable owner@id/repo@id format"
  }
  assert {
    condition     = toset(one([for c in data.aws_iam_policy_document.ci_plan_trust.statement[0].condition : c if c.variable == "token.actions.githubusercontent.com:sub"]).values) == toset(["repo:denmarkamanodaya@5304296/kristoff@1454246629:pull_request"])
    error_message = "ci-plan trusts exactly the pull_request subject"
  }
  assert {
    condition     = alltrue([for t in [data.aws_iam_policy_document.ci_deploy_trust, data.aws_iam_policy_document.ci_plan_trust] : toset(one([for c in t.statement[0].condition : c if c.variable == "token.actions.githubusercontent.com:aud"]).values) == toset(["sts.amazonaws.com"])])
    error_message = "both roles require the sts.amazonaws.com audience"
  }
  assert {
    condition     = alltrue([for t in [data.aws_iam_policy_document.ci_deploy_trust, data.aws_iam_policy_document.ci_plan_trust] : alltrue([for c in t.statement[0].condition : !anytrue([for v in c.values : strcontains(v, "*")])])])
    error_message = "no trust condition contains a wildcard"
  }
}

run "lambda_boundary_is_the_stage_names_ceiling" {
  command = plan

  assert {
    condition     = [for s in data.aws_iam_policy_document.lambda_boundary.statement : s.sid if contains(s.resources, "*")] == ["XRay"]
    error_message = "only X-Ray (no resource-level permissions) uses a wildcard resource"
  }
  assert {
    condition     = !anytrue([for s in data.aws_iam_policy_document.lambda_boundary.statement : anytrue([for a in s.actions : strcontains(a, "*")])])
    error_message = "the boundary lists enumerated actions only"
  }
  assert {
    condition     = one([for s in data.aws_iam_policy_document.lambda_boundary.statement : s if s.sid == "S3DocObjects"]).resources == toset(["arn:aws:s3:::kristoff-prod-docs-123456789012/docs/*"])
    error_message = "object actions only under docs/ of the docs bucket"
  }
  assert {
    condition     = one([for s in data.aws_iam_policy_document.lambda_boundary.statement : s if s.sid == "S3DocVersionsList"]).resources == toset(["arn:aws:s3:::kristoff-prod-docs-123456789012"]) && toset(one(one([for s in data.aws_iam_policy_document.lambda_boundary.statement : s if s.sid == "S3DocVersionsList"]).condition).values) == toset(["docs/*"])
    error_message = "ListBucketVersions only on the bucket and only for keys under docs/"
  }
  assert {
    condition     = one([for s in data.aws_iam_policy_document.lambda_boundary.statement : s if s.sid == "InvokeStageFunctions"]).resources == toset(["arn:aws:lambda:ap-southeast-2:123456789012:function:kristoff-prod-*"])
    error_message = "a function may invoke only kristoff-prod-* functions"
  }
  assert {
    condition     = one([for s in data.aws_iam_policy_document.lambda_boundary.statement : s if s.sid == "SsmReadStageParameters"]).resources == toset(["arn:aws:ssm:ap-southeast-2:123456789012:parameter/kristoff/prod/*"])
    error_message = "parameter reads only under /kristoff/prod/"
  }
  assert {
    condition     = toset([for c in one([for s in data.aws_iam_policy_document.lambda_boundary.statement : s if s.sid == "KmsDecryptStageParametersViaSsm"]).condition : c.variable]) == toset(["kms:ViaService", "kms:EncryptionContext:PARAMETER_ARN"])
    error_message = "kms:Decrypt only through SSM and only for this stage's parameters"
  }
  assert {
    condition     = !contains(flatten([for s in data.aws_iam_policy_document.lambda_boundary.statement : s.actions]), "dynamodb:Scan")
    error_message = "no Scan (stack/data-and-migrations.md: never on a request path)"
  }
}

run "ci_deploy_is_data_blind_and_cannot_destroy_data_stores" {
  command = plan

  assert {
    condition = !anytrue([
      for d in [data.aws_iam_policy_document.ci_deploy_compute, data.aws_iam_policy_document.ci_deploy_data, data.aws_iam_policy_document.ci_deploy_iam] :
      anytrue([for s in d.statement : s.effect == "Allow" && anytrue([for a in s.actions : contains(["dynamodb:GetItem", "dynamodb:PutItem", "dynamodb:Query", "dynamodb:Scan", "lambda:InvokeFunction", "bedrock:InvokeModel"], a)])])
    ])
    error_message = "no ci-deploy Allow reads or writes owner data or invokes a function"
  }
  assert {
    condition = !anytrue([
      for d in [data.aws_iam_policy_document.ci_deploy_compute, data.aws_iam_policy_document.ci_deploy_data, data.aws_iam_policy_document.ci_deploy_iam] :
      anytrue([for s in d.statement : s.effect == "Allow" && anytrue([for a in s.actions : strcontains(a, "*")])])
    ])
    error_message = "every ci-deploy Allow action is enumerated (no action wildcard)"
  }
  assert {
    condition     = contains(flatten([for s in data.aws_iam_policy_document.ci_deploy_deny.statement : s.actions]), "dynamodb:DeleteTable") && contains(flatten([for s in data.aws_iam_policy_document.ci_deploy_deny.statement : s.actions]), "s3:DeleteBucket")
    error_message = "the deny policy blocks deleting the table and buckets"
  }
  assert {
    condition     = one([for s in data.aws_iam_policy_document.ci_deploy_deny.statement : s if s.sid == "DenyDocObjects"]).resources == toset(["arn:aws:s3:::kristoff-prod-docs-123456789012/*"])
    error_message = "the deploy role can never read, write or delete a PDF"
  }
  assert {
    condition     = alltrue([for s in data.aws_iam_policy_document.ci_deploy_deny.statement : s.effect == "Deny"])
    error_message = "every statement of the deny policy is a Deny"
  }
  assert {
    condition     = length(aws_iam_policy.ci_deploy) == 4
    error_message = "four ci-deploy policies: compute, data, iam, deny"
  }
}

run "ci_roles_reach_only_the_kill_switch_in_ssm" {
  command = plan

  assert {
    condition     = one([for s in data.aws_iam_policy_document.ci_deploy_data.statement : s if s.sid == "SsmKillSwitch"]).resources == toset(["arn:aws:ssm:ap-southeast-2:123456789012:parameter/kristoff/prod/ai-enabled"])
    error_message = "the SSM allow names exactly the kill switch"
  }
  assert {
    condition     = one([for s in data.aws_iam_policy_document.ci_deploy_deny.statement : s if s.sid == "DenySsmExceptKillSwitch"]).not_resources == toset(["arn:aws:ssm:ap-southeast-2:123456789012:parameter/kristoff/prod/ai-enabled"])
    error_message = "every other parameter (the SecureStrings) stays denied for ci-deploy"
  }
  assert {
    condition     = one([for s in data.aws_iam_policy_document.ci_plan.statement : s if s.sid == "DenySsmReadsExceptKillSwitch"]).not_resources == toset(["arn:aws:ssm:ap-southeast-2:123456789012:parameter/kristoff/prod/ai-enabled"])
    error_message = "every other parameter stays denied for ci-plan"
  }
}

run "ci_deploy_manages_amplify_and_function_urls" {
  command = plan

  assert {
    condition     = alltrue([for a in ["amplify:CreateDeployment", "amplify:StartDeployment", "amplify:GetJob", "amplify:CreateBranch"] : contains(one([for s in data.aws_iam_policy_document.ci_deploy_data.statement : s if s.sid == "AmplifyManage"]).actions, a)])
    error_message = "the deploy role can manage the app and upload deployments"
  }
  assert {
    condition     = !anytrue([for a in one([for s in data.aws_iam_policy_document.ci_deploy_data.statement : s if s.sid == "AmplifyManage"]).actions : strcontains(a, "DomainAssociation")])
    error_message = "no custom-domain actions (owner ruling: Amplify default domain only)"
  }
  assert {
    condition     = one([for s in data.aws_iam_policy_document.ci_deploy_data.statement : s if s.sid == "AmplifyCreate"]).actions == toset(["amplify:CreateApp", "amplify:ListApps"])
    error_message = "only the two Amplify actions without a resource type use *"
  }
  assert {
    condition     = contains(one([for s in data.aws_iam_policy_document.ci_deploy_compute.statement : s if s.sid == "Lambda"]).actions, "lambda:CreateFunctionUrlConfig")
    error_message = "the deploy role manages the ask-stream function URL"
  }
}

run "state_bucket_is_private_versioned_and_kms_encrypted" {
  command = plan

  assert {
    condition     = aws_s3_bucket.tfstate.bucket == "kristoff-tfstate-123456789012"
    error_message = "state bucket name is the stage-names one"
  }
  assert {
    condition     = aws_s3_bucket_versioning.tfstate.versioning_configuration[0].status == "Enabled"
    error_message = "state versioning is on"
  }
  assert {
    condition     = one(aws_s3_bucket_server_side_encryption_configuration.tfstate.rule).apply_server_side_encryption_by_default[0].sse_algorithm == "aws:kms"
    error_message = "state is SSE-KMS"
  }
  assert {
    condition     = alltrue([aws_s3_bucket_public_access_block.tfstate.block_public_acls, aws_s3_bucket_public_access_block.tfstate.block_public_policy, aws_s3_bucket_public_access_block.tfstate.ignore_public_acls, aws_s3_bucket_public_access_block.tfstate.restrict_public_buckets])
    error_message = "state Block Public Access is on"
  }
}

run "guardduty_on_when_asked" {
  command = plan

  assert {
    condition     = length(aws_guardduty_detector.this) == 1
    error_message = "prod enables GuardDuty"
  }
}

run "budget_notifies_the_owner" {
  command = plan

  assert {
    condition     = aws_budgets_budget.monthly.limit_amount == "20" && length(aws_budgets_budget.monthly.notification) == 3
    error_message = "a $20 monthly budget with three notifications"
  }
}

run "rejects_a_malformed_account_id" {
  command = plan

  variables {
    account_id = "prod-account"
  }

  expect_failures = [var.account_id]
}
