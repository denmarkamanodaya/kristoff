# env/prod.platform.backend.hcl — the state of the platform stack in the kristoff-prod account. Used ONLY by
# backend/terraform/tf.sh prod platform init (it pairs this file with env/prod.tfvars). Lock object = <key>.tflock (no
# DynamoDB table). The stage guard (modules/stage-guard/v1) fails the plan unless bucket = kristoff-tfstate-<account_id>
# and key = kristoff/prod/platform.tfstate for the stage env/prod.tfvars claims.

bucket       = "kristoff-tfstate-815882254506"
key          = "kristoff/prod/platform.tfstate"
region       = "ap-southeast-2"
encrypt      = true
use_lockfile = true
