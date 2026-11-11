# roots/platform — the one root of the platform stack for every stage. Run ONLY through
# backend/terraform/tf.sh <stage> platform <cmd>: it pairs env/<stage>.platform.backend.hcl with env/<stage>.tfvars +
# env/<stage>.platform.tfvars and sets TF_DATA_DIR=.terraform-<stage>.

provider "aws" {
  region = var.region
  default_tags {
    tags = {
      Project   = "kristoff"
      Stage     = var.stage
      Stack     = "platform"
      ManagedBy = "terraform"
      Repo      = "denmarkamanodaya/kristoff"
    }
  }
}

module "stage_guard" {
  source = "../../modules/stage-guard/v1"

  stack              = "platform"
  stage              = var.stage
  account_id         = var.account_id
  backend_state_json = try(file("${path.root}/.terraform-${var.stage}/terraform.tfstate"), "")
}

module "platform" {
  source = "../../stacks/platform"

  stage       = var.stage
  account_id  = var.account_id
  region      = var.region
  alert_email = var.alert_email

  # No depends_on on module.stage_guard, on purpose: a failed guard precondition already fails the whole plan, and a
  # pending change inside the guard (the one-time identity create) would defer every data source of this stack to
  # apply time.
}
