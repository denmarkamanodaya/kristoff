# ci-deploy policy "iam": Lambda execution roles under the Lambda path capped by the permission boundary.
# Traps: the boundary condition key is present only on CreateRole, PutRolePolicy, AttachRolePolicy and
# PutRolePermissionsBoundary — a condition on the other actions would deny them. Managed policies are deliberately
# not granted: the lambda-fn module uses one inline policy per role and the deny policy forbids attaching any.

data "aws_iam_policy_document" "ci_deploy_iam" {
  statement {
    sid       = "LambdaRolesBounded"
    effect    = "Allow"
    actions   = ["iam:CreateRole", "iam:PutRolePolicy", "iam:PutRolePermissionsBoundary"]
    resources = ["arn:aws:iam::${var.account_id}:role${local.lambda_role_path}*"]
    condition {
      test     = "StringEquals"
      variable = "iam:PermissionsBoundary"
      values   = [aws_iam_policy.lambda_boundary.arn]
    }
  }

  statement {
    sid    = "LambdaRolesManage"
    effect = "Allow"
    actions = [
      "iam:DeleteRole", "iam:GetRole", "iam:UpdateRole", "iam:UpdateRoleDescription", "iam:UpdateAssumeRolePolicy",
      "iam:DeleteRolePolicy", "iam:GetRolePolicy", "iam:ListRolePolicies",
      "iam:DetachRolePolicy", "iam:ListAttachedRolePolicies",
      "iam:TagRole", "iam:UntagRole", "iam:ListRoleTags", "iam:ListInstanceProfilesForRole",
    ]
    resources = ["arn:aws:iam::${var.account_id}:role${local.lambda_role_path}*"]
  }

  statement {
    sid       = "PassLambdaRoles"
    effect    = "Allow"
    actions   = ["iam:PassRole"]
    resources = ["arn:aws:iam::${var.account_id}:role${local.lambda_role_path}*"]
    condition {
      test     = "StringEquals"
      variable = "iam:PassedToService"
      values   = ["lambda.amazonaws.com"]
    }
  }
}
