# lambda-stream-url v1
Since 2026-11-12 · first version

Purpose: One streaming Lambda function (built by `lambda-fn/v1`: role, inline policy, log group, published version, `live` alias) behind a public function URL on the `live` alias with `invoke_mode = RESPONSE_STREAM`, CORS (POST, headers `authorization` + `content-type`, exposes `retry-after`, exact origins) and the two resource-policy statements a public URL needs (`lambda:InvokeFunctionUrl` for auth type NONE, `lambda:InvokeFunction` with `invoked_via_function_url`).

Owns: the `lambda-fn/v1` call, `aws_lambda_function_url`, two `aws_lambda_permission`. The URL has no IAM auth: the function authenticates every request itself (Kristoff: `ask-stream` verifies the session token, `docs/contract.md`).

Rollback: n/a — first version. Bundle rollback = point the `live` alias at the previous published version; the URL follows the alias.

Test: `terraform test` here (plan mode, offline; fixture bundle under `tests/fixtures/`).

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

## Modules

| Name | Source | Version |
| ---- | ------ | ------- |
| <a name="module_fn"></a> [fn](#module\_fn) | ../../lambda-fn/v1 | n/a |

## Resources

| Name | Type |
| ---- | ---- |
| [aws_lambda_function_url.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/lambda_function_url) | resource |
| [aws_lambda_permission.invoke_via_url](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/lambda_permission) | resource |
| [aws_lambda_permission.url](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/lambda_permission) | resource |

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_architecture"></a> [architecture](#input\_architecture) | Instruction set architecture (validated by lambda-fn v1). | `string` | `"arm64"` | no |
| <a name="input_cors_allowed_origins"></a> [cors\_allowed\_origins](#input\_cors\_allowed\_origins) | Exact browser origins the URL's CORS allows (https://host[:port], or http://localhost:<port>); never *. Methods are POST, headers authorization and content-type, Retry-After exposed. | `list(string)` | n/a | yes |
| <a name="input_description"></a> [description](#input\_description) | Human description shown in the Lambda console. | `string` | `""` | no |
| <a name="input_env"></a> [env](#input\_env) | Environment variables (names and non-secret values only; secrets are read at runtime by name). | `map(string)` | `{}` | no |
| <a name="input_handler"></a> [handler](#input\_handler) | Handler export, <file>.<function> of the ESM bundle. | `string` | `"index.handler"` | no |
| <a name="input_log_retention_days"></a> [log\_retention\_days](#input\_log\_retention\_days) | CloudWatch log retention in days (validated by lambda-fn v1). | `number` | `30` | no |
| <a name="input_memory_mb"></a> [memory\_mb](#input\_memory\_mb) | Memory in MB (validated by lambda-fn v1). | `number` | `512` | no |
| <a name="input_name"></a> [name](#input\_name) | Fully prefixed function name, e.g. kristoff-prod-ask-stream; also the role name and the log group suffix. | `string` | n/a | yes |
| <a name="input_permissions_boundary_arn"></a> [permissions\_boundary\_arn](#input\_permissions\_boundary\_arn) | ARN of the permission boundary policy the role must carry (validated by lambda-fn v1). | `string` | n/a | yes |
| <a name="input_policy_json"></a> [policy\_json](#input\_policy\_json) | IAM policy document (JSON) with the function's own statements; lambda-fn v1 adds the logs statement. | `string` | n/a | yes |
| <a name="input_reserved_concurrency"></a> [reserved\_concurrency](#input\_reserved\_concurrency) | Reserved concurrent executions applied when `reserved_concurrency_enabled` is true. | `number` | `5` | no |
| <a name="input_reserved_concurrency_enabled"></a> [reserved\_concurrency\_enabled](#input\_reserved\_concurrency\_enabled) | Apply `reserved_concurrency` (stays false while the account quota is the new-account default). | `bool` | `false` | no |
| <a name="input_role_path"></a> [role\_path](#input\_role\_path) | IAM path of the execution role (validated by lambda-fn v1). | `string` | n/a | yes |
| <a name="input_runtime"></a> [runtime](#input\_runtime) | Lambda runtime identifier (validated by lambda-fn v1). | `string` | `"nodejs24.x"` | no |
| <a name="input_tags"></a> [tags](#input\_tags) | Extra tags; the provider's default\_tags carry Project/Stage/Stack/ManagedBy/Repo, lambda-fn adds Name. | `map(string)` | `{}` | no |
| <a name="input_timeout_s"></a> [timeout\_s](#input\_timeout\_s) | Timeout in seconds (1-900); a function URL has no 30 s gateway cut-off, so a stream may run up to this. | `number` | `300` | no |
| <a name="input_zip_path"></a> [zip\_path](#input\_zip\_path) | Path to the deployment bundle (backend/dist/<function key>.zip, or the stack's placeholder bundle). | `string` | n/a | yes |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_alias_arn"></a> [alias\_arn](#output\_alias\_arn) | ARN of the `live` alias the URL serves. |
| <a name="output_env_keys"></a> [env\_keys](#output\_env\_keys) | Sorted environment variable keys set on the function. |
| <a name="output_function_name"></a> [function\_name](#output\_function\_name) | Function name. |
| <a name="output_function_url"></a> [function\_url](#output\_function\_url) | Public HTTPS URL of the `live` alias (https://<url id>.lambda-url.<region>.on.aws/), ending in /. |
| <a name="output_granted_actions"></a> [granted\_actions](#output\_granted\_actions) | Sorted, distinct IAM actions the role's inline policy allows; known at plan time. |
| <a name="output_log_group_name"></a> [log\_group\_name](#output\_log\_group\_name) | CloudWatch log group name. |
| <a name="output_role_name"></a> [role\_name](#output\_role\_name) | Execution role name. |
<!-- END_TF_DOCS -->
