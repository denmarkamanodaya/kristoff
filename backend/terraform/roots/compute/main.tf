# roots/compute — the one root of the compute stack for every stage. Run ONLY through
# backend/terraform/tf.sh <stage> compute <cmd> (after the platform stack is applied): it pairs
# env/<stage>.compute.backend.hcl with env/<stage>.tfvars + env/<stage>.compute.tfvars and sets
# TF_DATA_DIR=.terraform-<stage>. It reads the platform outputs from the platform state. Bundles and the scope file
# come from the backend build (backend/dist, backend/lambda-scope.json; docs/contract.md "Build artefacts").

provider "aws" {
  region = var.region
  default_tags {
    tags = {
      Project   = "kristoff"
      Stage     = var.stage
      Stack     = "compute"
      ManagedBy = "terraform"
      Repo      = "denmarkamanodaya/kristoff"
    }
  }
}

module "stage_guard" {
  source = "../../modules/stage-guard/v1"

  stack              = "compute"
  stage              = var.stage
  account_id         = var.account_id
  backend_state_json = try(file("${path.root}/.terraform-${var.stage}/terraform.tfstate"), "")
}

# The web origin is the Amplify app's default domain, random and known only after the platform apply, so it cannot be
# built from stage-names. It is read from the platform stack's own state (same bucket, key kristoff/<stage>/platform.tfstate,
# both from stage-names); the platform stack must be applied first (deploy-prod.yml runs it first).
module "names" {
  source = "../../modules/stage-names/v1"

  stage      = var.stage
  account_id = var.account_id
  region     = var.region
}

data "terraform_remote_state" "platform" {
  backend = "s3"
  config = {
    bucket  = module.names.tfstate_bucket_name
    key     = "${module.names.tfstate_key_prefix}/platform.tfstate"
    region  = var.region
    encrypt = true
  }
}

module "compute" {
  source = "../../stacks/compute"

  stage                = var.stage
  account_id           = var.account_id
  region               = var.region
  web_origin           = data.terraform_remote_state.platform.outputs.web_origin
  lambda_zip_dir       = "${path.root}/../../../dist"
  lambda_scope_path    = "${path.root}/../../../lambda-scope.json"
  owner_id             = var.owner_id
  model_ids            = var.model_ids
  max_tokens           = var.max_tokens
  daily_question_cap   = var.daily_question_cap
  anthropic_timeout_ms = var.anthropic_timeout_ms
  followup_model_id    = var.followup_model_id

  # No depends_on on module.stage_guard: see roots/platform/main.tf.
}
