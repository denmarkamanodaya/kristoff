# Bootstrap account module — derived names and patterns. Names come from stage-names (one source with the stacks).

module "names" {
  source = "../../modules/stage-names/v1"

  stage      = var.stage
  account_id = var.account_id
  region     = var.region
}

locals {
  name_prefix       = module.names.prefix
  ci_role_path      = module.names.ci_role_path
  lambda_role_path  = module.names.lambda_role_path
  oidc_subject_base = "repo:${var.github_owner}@${var.github_owner_id}/${var.github_repo}@${var.github_repo_id}"

  # Everything the stacks create is named kristoff-<stage>-*; the account hosts one stage, so these are the narrowest
  # patterns AWS allows where an ARN cannot carry the name (random API and Amplify app ids).
  log_group_arn_pattern = "arn:aws:logs:${var.region}:${var.account_id}:log-group"
  amplify_app_arns      = ["arn:aws:amplify:${var.region}:${var.account_id}:apps/*"]
  amplify_child_arns = [
    "arn:aws:amplify:${var.region}:${var.account_id}:apps/*/branches/*",
    "arn:aws:amplify:${var.region}:${var.account_id}:apps/*/branches/*/jobs/*",
  ]

  # The one SSM parameter CI may touch: the AI kill switch the platform stack creates (String, value owner-flipped).
  # Every other parameter of the account, the owner-set SecureStrings above all, is denied in ci_deploy_policy_deny.tf.
  ci_ssm_config_param_arns = [module.names.ai_enabled_param_arn]

  # The Identity Center role name is generated as AWSReservedSSO_<permission set>_<random>; wildcard the suffix.
  sso_role_arn_pattern = "arn:aws:iam::${var.account_id}:role/aws-reserved/sso.amazonaws.com/*/AWSReservedSSO_${var.sso_permission_set_name}_*"
  # OrganizationAccountAccessRole is the documented break-glass path.
  break_glass_role_arn = "arn:aws:iam::${var.account_id}:role/OrganizationAccountAccessRole"
}
