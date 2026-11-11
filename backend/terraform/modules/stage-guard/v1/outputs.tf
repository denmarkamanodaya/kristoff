# stage-guard v1 — outputs.

output "account_id" {
  description = "The checked account id (equals the caller's account once the plan passes)."
  value       = var.account_id
}

output "state_key" {
  description = "The state key the guard required: kristoff/<stage>/<stack>.tfstate."
  value       = local.state_key
}

output "identity" {
  description = "The {stack, stage, account_id} recorded in this state (planned value = the value already in the state)."
  value       = terraform_data.identity.output
}
