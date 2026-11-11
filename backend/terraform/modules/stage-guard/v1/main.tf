# stage-guard v1 — fails the plan of a root (roots/<stack>) when the stage inputs, the credentials and the initialised
# backend disagree. Every root calls it. Three independent facts are compared with the stage the tfvars claim:
#   1. the caller account (data.aws_caller_identity) must equal var.account_id;
#   2. the backend Terraform actually initialised (the JSON `terraform init` wrote to <data dir>/terraform.tfstate,
#      passed in by the root as var.backend_state_json) must be the S3 bucket kristoff-tfstate-<account_id> with the
#      key kristoff/<stage>/<stack>.tfstate, native lockfile and encryption on;
#   3. the STATE itself: terraform_data.identity records {stack, stage, account_id} at the first apply into a state
#      and never changes (ignore_changes), so its PLANNED output is the value already in the state. A root planned
#      against another stack's or stage's state fails the precondition below.
# So tfvars paired with a foreign backend fail here even if tf.sh is bypassed.
# Trap: the guard is a terraform_data resource on purpose — preconditions run at plan time.

module "names" {
  source = "../../stage-names/v1"

  stage      = var.stage
  account_id = var.account_id
}

data "aws_caller_identity" "current" {}

locals {
  identity       = { stack = var.stack, stage = var.stage, account_id = var.account_id }
  backend        = try(jsondecode(var.backend_state_json).backend, null)
  backend_config = try(local.backend.config, {})
  state_bucket   = module.names.tfstate_bucket_name
  state_key      = "${module.names.tfstate_key_prefix}/${var.stack}.tfstate"
}

# The identity of the state this root plans against: written once, never updated (an update would let a foreign root
# re-label the state). Deleting it is a destroy the plan guard blocks (scripts/tf-plan-guard.jq).
resource "terraform_data" "identity" {
  input = local.identity

  lifecycle {
    ignore_changes = [input]
  }
}

resource "terraform_data" "this" {
  lifecycle {
    precondition {
      condition     = try(terraform_data.identity.output.stack, "") == var.stack && try(terraform_data.identity.output.stage, "") == var.stage && try(terraform_data.identity.output.account_id, "") == var.account_id
      error_message = "roots/${var.stack} (${var.stage}, ${var.account_id}) is planning against the state of ${try(terraform_data.identity.output.stack, "?")} (${try(terraform_data.identity.output.stage, "?")}, ${try(terraform_data.identity.output.account_id, "?")}): the initialised backend belongs to another stack or stage. Re-initialise with backend/terraform/tf.sh."
    }
    precondition {
      condition     = can(regex("^[0-9]{12}$", var.account_id))
      error_message = "roots/${var.stack}: account_id in env/${var.stage}.tfvars must be the 12-digit kristoff-${var.stage} account id (docs/runbooks/owner-setup.md)."
    }
    precondition {
      condition     = data.aws_caller_identity.current.account_id == var.account_id
      error_message = "roots/${var.stack} with env/${var.stage}.tfvars is the kristoff-${var.stage} account; the current credentials belong to another account (check AWS_PROFILE)."
    }
    precondition {
      condition     = try(local.backend.type, "") == "s3"
      error_message = "roots/${var.stack}: no initialised S3 backend found for stage ${var.stage}; run `backend/terraform/tf.sh ${var.stage} ${var.stack} init` (TF_DATA_DIR must be .terraform-${var.stage})."
    }
    precondition {
      condition     = try(local.backend_config.bucket, "") == local.state_bucket && try(local.backend_config.key, "") == local.state_key
      error_message = "roots/${var.stack}: the initialised backend is ${try(local.backend_config.bucket, "?")}/${try(local.backend_config.key, "?")}, but env/${var.stage}.tfvars needs ${local.state_bucket}/${local.state_key} — the tfvars and the backend belong to different stages or stacks."
    }
    precondition {
      condition     = try(local.backend_config.use_lockfile, false) == true && try(local.backend_config.encrypt, false) == true
      error_message = "roots/${var.stack}: the backend must set use_lockfile = true and encrypt = true (env/${var.stage}.${var.stack}.backend.hcl)."
    }
  }
}
