# lambda-fn v1
Since 2026-11-12 · first version (adapted from dexfs lambda-fn v1)

Purpose: One Lambda function with its execution role (inline policy = caller statements + logs; X-Ray only when `tracing_mode = Active`; DLQ send when enabled; permission boundary), 14-day log group, published version + `live` alias, optional reserved concurrency, SQS dead-letter queue and asynchronous retry count on the alias.

Owns: `aws_iam_role`, `aws_iam_role_policy`, `aws_cloudwatch_log_group`, `aws_lambda_function` (`publish = true`), `aws_lambda_alias` `live`, optional `aws_sqs_queue` DLQ, optional `aws_lambda_function_event_invoke_config` on `live`. Caller responsibilities: triggers and invoke permissions (`aws_lambda_permission`, routes, event-source mappings) and the function's own statements (`policy_json`).

Rollback: n/a — first version.

Test: `terraform test` here (plan mode, offline; fixture bundle under `tests/fixtures/`). Rollback of a bundle = point the `live` alias at the previous published version.

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

No modules.

## Resources

| Name | Type |
| ---- | ---- |
| [aws_cloudwatch_log_group.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/cloudwatch_log_group) | resource |
| [aws_iam_role.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/iam_role) | resource |
| [aws_iam_role_policy.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/iam_role_policy) | resource |
| [aws_lambda_alias.live](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/lambda_alias) | resource |
| [aws_lambda_function.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/lambda_function) | resource |
| [aws_lambda_function_event_invoke_config.live](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/lambda_function_event_invoke_config) | resource |
| [aws_sqs_queue.dlq](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/sqs_queue) | resource |
| [aws_iam_policy_document.assume](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/data-sources/iam_policy_document) | data source |
| [aws_iam_policy_document.baseline](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/data-sources/iam_policy_document) | data source |
| [aws_iam_policy_document.role](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/data-sources/iam_policy_document) | data source |

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_architecture"></a> [architecture](#input\_architecture) | Instruction set architecture. | `string` | `"arm64"` | no |
| <a name="input_async_retry_attempts"></a> [async\_retry\_attempts](#input\_async\_retry\_attempts) | Retries Lambda makes of a failed asynchronous invocation of the `live` alias (0-2); null leaves Lambda's default (2) unmanaged. Scheduled jobs set 0: the next scheduled run is the retry. | `number` | `null` | no |
| <a name="input_description"></a> [description](#input\_description) | Human description shown in the Lambda console. | `string` | `""` | no |
| <a name="input_dlq_enabled"></a> [dlq\_enabled](#input\_dlq\_enabled) | Create an SQS dead-letter queue <name>-dlq for failed asynchronous invocations and grant sqs:SendMessage on it. | `bool` | `false` | no |
| <a name="input_env"></a> [env](#input\_env) | Environment variables of the function (names and non-secret values only; secrets are read at runtime by name). | `map(string)` | `{}` | no |
| <a name="input_handler"></a> [handler](#input\_handler) | Handler export, <file>.<function> of the ESM bundle. | `string` | `"index.handler"` | no |
| <a name="input_log_retention_days"></a> [log\_retention\_days](#input\_log\_retention\_days) | CloudWatch log retention in days. | `number` | `14` | no |
| <a name="input_memory_mb"></a> [memory\_mb](#input\_memory\_mb) | Memory in MB (128-10240); CPU scales with it. | `number` | `256` | no |
| <a name="input_name"></a> [name](#input\_name) | Fully prefixed function name, e.g. kristoff-prod-health; also the role name and the log group suffix. | `string` | n/a | yes |
| <a name="input_permissions_boundary_arn"></a> [permissions\_boundary\_arn](#input\_permissions\_boundary\_arn) | ARN of the permission boundary policy every Lambda role must carry (kristoff-<stage>-lambda-boundary). | `string` | n/a | yes |
| <a name="input_policy_json"></a> [policy\_json](#input\_policy\_json) | IAM policy document (JSON) with the function's own statements; merged with the module's logs (and X-Ray/DLQ when enabled) statements into one inline role policy. | `string` | n/a | yes |
| <a name="input_reserved_concurrency"></a> [reserved\_concurrency](#input\_reserved\_concurrency) | Reserved concurrent executions applied when `reserved_concurrency_enabled` is true. | `number` | `20` | no |
| <a name="input_reserved_concurrency_enabled"></a> [reserved\_concurrency\_enabled](#input\_reserved\_concurrency\_enabled) | Apply `reserved_concurrency`; stays false while the account quota is the new-account default of 10 (AWS keeps 100 unreserved). | `bool` | `false` | no |
| <a name="input_role_path"></a> [role\_path](#input\_role\_path) | IAM path of the execution role; the deploy role may only manage roles under the Lambda path. | `string` | n/a | yes |
| <a name="input_runtime"></a> [runtime](#input\_runtime) | Lambda runtime identifier. | `string` | `"nodejs24.x"` | no |
| <a name="input_tags"></a> [tags](#input\_tags) | Extra tags; the provider's default\_tags carry Project/Stage/Stack/ManagedBy/Repo, the module adds Name. | `map(string)` | `{}` | no |
| <a name="input_timeout_s"></a> [timeout\_s](#input\_timeout\_s) | Timeout in seconds (1-900); HTTP functions use 29 so the handler answers before API Gateway's 30 s cut-off. | `number` | `5` | no |
| <a name="input_tracing_mode"></a> [tracing\_mode](#input\_tracing\_mode) | X-Ray tracing mode. | `string` | `"PassThrough"` | no |
| <a name="input_zip_path"></a> [zip\_path](#input\_zip\_path) | Path to the deployment bundle (backend/dist/<function key>.zip, or the stack's placeholder bundle). | `string` | n/a | yes |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_alias_arn"></a> [alias\_arn](#output\_alias\_arn) | ARN of the `live` alias (what triggers must target). |
| <a name="output_alias_invoke_arn"></a> [alias\_invoke\_arn](#output\_alias\_invoke\_arn) | Invoke ARN of the `live` alias for API Gateway integrations. |
| <a name="output_alias_name"></a> [alias\_name](#output\_alias\_name) | Name of the alias (`live`), the qualifier for aws\_lambda\_permission. |
| <a name="output_async_retry_attempts"></a> [async\_retry\_attempts](#output\_async\_retry\_attempts) | Retries of a failed asynchronous invocation of the `live` alias, or null when Lambda's default is left unmanaged. |
| <a name="output_dlq_arn"></a> [dlq\_arn](#output\_dlq\_arn) | Dead-letter queue ARN, or null when `dlq_enabled` is false. |
| <a name="output_env_keys"></a> [env\_keys](#output\_env\_keys) | Sorted environment variable keys set on the function. |
| <a name="output_function_arn"></a> [function\_arn](#output\_function\_arn) | Unqualified function ARN. |
| <a name="output_function_name"></a> [function\_name](#output\_function\_name) | Function name. |
| <a name="output_granted_actions"></a> [granted\_actions](#output\_granted\_actions) | Sorted, distinct IAM actions allowed by the role's inline policy (module statements + policy\_json); known at plan time. |
| <a name="output_log_group_name"></a> [log\_group\_name](#output\_log\_group\_name) | CloudWatch log group name. |
| <a name="output_published_version"></a> [published\_version](#output\_published\_version) | Version number published by this apply. |
| <a name="output_role_arn"></a> [role\_arn](#output\_role\_arn) | Execution role ARN. |
| <a name="output_role_name"></a> [role\_name](#output\_role\_name) | Execution role name. |
<!-- END_TF_DOCS -->
