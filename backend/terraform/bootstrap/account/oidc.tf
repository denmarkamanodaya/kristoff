# GitHub Actions OIDC provider and the trust policies for the two CI roles.
# Trap: the subject uses GitHub's IMMUTABLE format (owner@id/repo@id — mandatory for repositories created after
# 2026-07-15; kristoff was created 2026-11-12; the plain repo:OWNER/REPO form never matches). A
# workflow must declare `environment: <stage>` (deploy) or run on `pull_request` (plan), or the subject never matches.
# A refused run shows the presented subject in CloudTrail AssumeRoleWithWebIdentity -> userIdentity.principalId.

resource "aws_iam_openid_connect_provider" "github" {
  url            = "https://token.actions.githubusercontent.com"
  client_id_list = ["sts.amazonaws.com"]
  # AWS validates GitHub's issuer against its trusted CA store; the thumbprints are kept for older clients.
  thumbprint_list = [
    "6938fd4d98bab03faadb97b34396831e3780aea1",
    "1c58a3a8518e8759bf075b76b750d4f2df264fcd",
  ]
  tags = { Name = "${local.name_prefix}-github-oidc" }
}

data "aws_iam_policy_document" "ci_deploy_trust" {
  statement {
    sid     = "GitHubEnvironment"
    effect  = "Allow"
    actions = ["sts:AssumeRoleWithWebIdentity"]
    principals {
      type        = "Federated"
      identifiers = [aws_iam_openid_connect_provider.github.arn]
    }
    condition {
      test     = "StringEquals"
      variable = "token.actions.githubusercontent.com:aud"
      values   = ["sts.amazonaws.com"]
    }
    condition {
      test     = "StringEquals"
      variable = "token.actions.githubusercontent.com:sub"
      values   = ["${local.oidc_subject_base}:environment:${var.stage}"]
    }
  }
}

data "aws_iam_policy_document" "ci_plan_trust" {
  statement {
    sid     = "GitHubPullRequest"
    effect  = "Allow"
    actions = ["sts:AssumeRoleWithWebIdentity"]
    principals {
      type        = "Federated"
      identifiers = [aws_iam_openid_connect_provider.github.arn]
    }
    condition {
      test     = "StringEquals"
      variable = "token.actions.githubusercontent.com:aud"
      values   = ["sts.amazonaws.com"]
    }
    condition {
      test     = "StringEquals"
      variable = "token.actions.githubusercontent.com:sub"
      values   = ["${local.oidc_subject_base}:pull_request"]
    }
  }
}
