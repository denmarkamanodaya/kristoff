# stage-names v1
Since 2026-11-12 · first version (adapted from dexfs stage-names v1)

Purpose: The one place that builds every stage-scoped name, ARN and shared constant of Kristoff (prefix, table, docs bucket, SSM parameter names, role paths, boundary ARN, alerts topic, Amplify app name, state and trail buckets). Names follow `docs/contract.md`.

No resources, no provider. Callers: `bootstrap/account`, `stacks/platform`, `stacks/compute`, `modules/stage-guard`. A name change here renames resources in every caller (the plan guard blocks replacing the table and the bucket).

Rollback: n/a — first version.

Test: `terraform test` here (plan mode, offline).

<!-- BEGIN_TF_DOCS -->
## Requirements

| Name | Version |
| ---- | ------- |
| <a name="requirement_terraform"></a> [terraform](#requirement\_terraform) | ~> 1.14 |

## Providers

No providers.

## Modules

No modules.

## Resources

No resources.

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_account_id"></a> [account\_id](#input\_account\_id) | 12-digit AWS account id of kristoff-<stage>. A TODO\_* marker passes validation so roots validate offline; the roots' preconditions refuse it at plan time. | `string` | n/a | yes |
| <a name="input_region"></a> [region](#input\_region) | AWS region of the stage (the region allow-list SCP admits ap-southeast-2 only). | `string` | `"ap-southeast-2"` | no |
| <a name="input_stage"></a> [stage](#input\_stage) | Deployment stage: prod (Kristoff has one stage). | `string` | n/a | yes |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_ai_enabled_param_arn"></a> [ai\_enabled\_param\_arn](#output\_ai\_enabled\_param\_arn) | ARN of the AI kill switch parameter. |
| <a name="output_ai_enabled_param_name"></a> [ai\_enabled\_param\_name](#output\_ai\_enabled\_param\_name) | SSM String name of the AI kill switch (true \| false; Terraform creates it, the owner flips it). |
| <a name="output_alerts_topic_arn"></a> [alerts\_topic\_arn](#output\_alerts\_topic\_arn) | Alerts SNS topic ARN. |
| <a name="output_alerts_topic_name"></a> [alerts\_topic\_name](#output\_alerts\_topic\_name) | Alerts SNS topic name. |
| <a name="output_amplify_app_arn_pattern"></a> [amplify\_app\_arn\_pattern](#output\_amplify\_app\_arn\_pattern) | ARN pattern of Amplify apps in the account (app ids are random; the account hosts one stage). |
| <a name="output_amplify_app_name"></a> [amplify\_app\_name](#output\_amplify\_app\_name) | Amplify Hosting app name of the web client. |
| <a name="output_anthropic_key_param_name"></a> [anthropic\_key\_param\_name](#output\_anthropic\_key\_param\_name) | SSM SecureString name of the Anthropic API key (owner-set, never in Terraform). |
| <a name="output_boundary_arn"></a> [boundary\_arn](#output\_boundary\_arn) | ARN of the Lambda permission boundary policy. |
| <a name="output_boundary_name"></a> [boundary\_name](#output\_boundary\_name) | Name of the Lambda permission boundary policy. |
| <a name="output_ci_role_path"></a> [ci\_role\_path](#output\_ci\_role\_path) | IAM path of the CI roles and the boundary policy. |
| <a name="output_cloudtrail_bucket_name"></a> [cloudtrail\_bucket\_name](#output\_cloudtrail\_bucket\_name) | CloudTrail log bucket of the account. |
| <a name="output_docs_bucket_arn"></a> [docs\_bucket\_arn](#output\_docs\_bucket\_arn) | Docs bucket ARN. |
| <a name="output_docs_bucket_name"></a> [docs\_bucket\_name](#output\_docs\_bucket\_name) | Private PDF bucket kristoff-<stage>-docs-<account id>. |
| <a name="output_docs_object_prefix"></a> [docs\_object\_prefix](#output\_docs\_object\_prefix) | Key prefix of every PDF object, docs/ (keys are docs/<uid>/<docId>.pdf). |
| <a name="output_docs_objects_arn"></a> [docs\_objects\_arn](#output\_docs\_objects\_arn) | ARN pattern of every PDF object, <bucket ARN>/docs/* (keys are docs/<uid>/<docId>.pdf, server-built). |
| <a name="output_lambda_dynamodb_actions"></a> [lambda\_dynamodb\_actions](#output\_lambda\_dynamodb\_actions) | DynamoDB item actions a Lambda role may hold on the table and its indexes (the boundary's ceiling). |
| <a name="output_lambda_function_arn_prefix"></a> [lambda\_function\_arn\_prefix](#output\_lambda\_function\_arn\_prefix) | ARN prefix of every function of the stage (append * or a function key). |
| <a name="output_lambda_role_path"></a> [lambda\_role\_path](#output\_lambda\_role\_path) | IAM path of every Lambda execution role (the deploy role may only manage this path). |
| <a name="output_lambda_s3_actions"></a> [lambda\_s3\_actions](#output\_lambda\_s3\_actions) | S3 object actions a Lambda role may hold on docs/* of the docs bucket (the boundary's ceiling). |
| <a name="output_log_group_prefix"></a> [log\_group\_prefix](#output\_log\_group\_prefix) | Prefix of non-Lambda log groups (API access logs). |
| <a name="output_prefix"></a> [prefix](#output\_prefix) | Resource name prefix kristoff-<stage>. |
| <a name="output_secret_param_arns"></a> [secret\_param\_arns](#output\_secret\_param\_arns) | ARNs of the three owner-set SecureString parameters (same order as secret\_param\_names). |
| <a name="output_secret_param_names"></a> [secret\_param\_names](#output\_secret\_param\_names) | Names of the three owner-set SecureString parameters, in the order anthropic key, TOTP secret, session key. |
| <a name="output_session_key_param_name"></a> [session\_key\_param\_name](#output\_session\_key\_param\_name) | SSM SecureString name of the session HMAC key (owner-set, never in Terraform). |
| <a name="output_ssm_parameter_arn_pattern"></a> [ssm\_parameter\_arn\_pattern](#output\_ssm\_parameter\_arn\_pattern) | ARN pattern of every SSM parameter of the stage (/kristoff/<stage>/*). |
| <a name="output_ssm_parameter_arn_prefix"></a> [ssm\_parameter\_arn\_prefix](#output\_ssm\_parameter\_arn\_prefix) | Prefix that turns an SSM name (with its leading slash) into its ARN: arn:aws:ssm:<region>:<account>:parameter. |
| <a name="output_table_arn"></a> [table\_arn](#output\_table\_arn) | DynamoDB table ARN (append /index/* for its indexes). |
| <a name="output_table_name"></a> [table\_name](#output\_table\_name) | DynamoDB table name kristoff-<stage>-core (docs/contract.md "Storage"). |
| <a name="output_tfstate_bucket_name"></a> [tfstate\_bucket\_name](#output\_tfstate\_bucket\_name) | Terraform state bucket of the account. |
| <a name="output_tfstate_key_prefix"></a> [tfstate\_key\_prefix](#output\_tfstate\_key\_prefix) | State key prefix of the stage: <prefix>/<stack>.tfstate. |
| <a name="output_totp_secret_param_name"></a> [totp\_secret\_param\_name](#output\_totp\_secret\_param\_name) | SSM SecureString name of the base32 TOTP secret (owner-set, never in Terraform). |
<!-- END_TF_DOCS -->
