# stage-guard v1
Since 2026-11-12 · first version (adapted from dexfs stage-guard v1)

Purpose: Fails the plan of a root when the stage tfvars, the caller's credentials and the initialised S3 backend disagree (account, bucket `kristoff-tfstate-<account>`, key `kristoff/<stage>/<stack>.tfstate`, lockfile, encryption) or when the state belongs to another stack or stage.

Owns: `terraform_data.identity` (written once, `ignore_changes`) and `terraform_data.this` (the preconditions). Caller responsibility: pass `backend_state_json = try(file("${path.root}/.terraform-<stage>/terraform.tfstate"), "")`.

Rollback: n/a — first version.

Test: `terraform test` here (plan mode, caller account overridden).

<!-- BEGIN_TF_DOCS -->
## Requirements

| Name | Version |
| ---- | ------- |
| <a name="requirement_terraform"></a> [terraform](#requirement\_terraform) | ~> 1.14 |
| <a name="requirement_aws"></a> [aws](#requirement\_aws) | ~> 6.67 |

## Providers

| Name | Version |
| ---- | ------- |
| <a name="provider_aws"></a> [aws](#provider\_aws) | 6.72.0 |
| <a name="provider_terraform"></a> [terraform](#provider\_terraform) | n/a |

## Modules

| Name | Source | Version |
| ---- | ------ | ------- |
| <a name="module_names"></a> [names](#module\_names) | ../../stage-names/v1 | n/a |

## Resources

| Name | Type |
| ---- | ---- |
| [terraform_data.identity](https://registry.terraform.io/providers/hashicorp/terraform/latest/docs/resources/data) | resource |
| [terraform_data.this](https://registry.terraform.io/providers/hashicorp/terraform/latest/docs/resources/data) | resource |
| [aws_caller_identity.current](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/data-sources/caller_identity) | data source |

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_account_id"></a> [account\_id](#input\_account\_id) | 12-digit AWS account id of kristoff-<stage> from env/<stage>.tfvars; the state bucket name embeds it. Not validated here on purpose: a malformed value must fail the plan with the guard's message, not a variable error. | `string` | n/a | yes |
| <a name="input_backend_state_json"></a> [backend\_state\_json](#input\_backend\_state\_json) | Contents of <TF\_DATA\_DIR>/terraform.tfstate, the backend record `terraform init` writes (type + config: bucket, key, use\_lockfile, encrypt), or "" when absent. The root reads it with try(file(...), ""); an empty or foreign value fails the plan. | `string` | n/a | yes |
| <a name="input_stack"></a> [stack](#input\_stack) | Stack of the calling root (roots/<stack>): platform or compute; part of the expected state key and every error message. | `string` | n/a | yes |
| <a name="input_stage"></a> [stage](#input\_stage) | Stage the tfvars claim: prod (Kristoff has one stage). | `string` | n/a | yes |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_account_id"></a> [account\_id](#output\_account\_id) | The checked account id (equals the caller's account once the plan passes). |
| <a name="output_identity"></a> [identity](#output\_identity) | The {stack, stage, account\_id} recorded in this state (planned value = the value already in the state). |
| <a name="output_state_key"></a> [state\_key](#output\_state\_key) | The state key the guard required: kristoff/<stage>/<stack>.tfstate. |
<!-- END_TF_DOCS -->
