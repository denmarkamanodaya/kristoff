# ci-deploy: the role deploy workflows assume through a GitHub environment (subject ...:environment:<stage>).
# Traps: four customer-managed policies (each <= 6,144 characters, checked by precondition); the explicit-deny policy is
# attached by name but IAM evaluates Deny first regardless of order. The policies are TRACED from the resources the
# stacks declare, not measured: the first deploy may surface an AccessDenied, which is fixed here and re-applied by
# the owner (Kristoff has no dev stage to try it on first).

resource "aws_iam_role" "ci_deploy" {
  name                 = "${local.name_prefix}-ci-deploy"
  path                 = local.ci_role_path
  description          = "GitHub Actions deploys for ${local.name_prefix} (environment ${var.stage})"
  assume_role_policy   = data.aws_iam_policy_document.ci_deploy_trust.json
  max_session_duration = 3600
  tags                 = { Name = "${local.name_prefix}-ci-deploy" }
}

locals {
  ci_deploy_policies = {
    compute = data.aws_iam_policy_document.ci_deploy_compute
    data    = data.aws_iam_policy_document.ci_deploy_data
    iam     = data.aws_iam_policy_document.ci_deploy_iam
    deny    = data.aws_iam_policy_document.ci_deploy_deny
  }
}

resource "aws_iam_policy" "ci_deploy" {
  for_each = local.ci_deploy_policies

  name        = "${local.name_prefix}-ci-deploy-${each.key}"
  path        = local.ci_role_path
  description = "ci-deploy ${each.key} permissions for ${local.name_prefix}"
  policy      = each.value.json
  tags        = { Name = "${local.name_prefix}-ci-deploy-${each.key}" }

  lifecycle {
    precondition {
      condition     = length(each.value.minified_json) <= 6144
      error_message = "ci-deploy ${each.key} policy exceeds the 6,144-character managed-policy limit; split it."
    }
  }
}

resource "aws_iam_role_policy_attachment" "ci_deploy" {
  for_each = aws_iam_policy.ci_deploy

  role       = aws_iam_role.ci_deploy.name
  policy_arn = each.value.arn
}
