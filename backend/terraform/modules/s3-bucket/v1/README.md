# s3-bucket v1
Since 2026-11-12 · first version (adapted from dexfs s3-bucket v1)

Purpose: A private, versioned bucket (Block Public Access, ACLs off, TLS-only policy, SSE-S3) with a lifecycle: abort incomplete multipart uploads, expire noncurrent versions, expire chosen prefixes (none by default) and clean delete markers; optional CORS for exact browser origins (presigned PUT/GET).

Owns: bucket, ownership controls, public access block, versioning, encryption, lifecycle, optional CORS rule, TLS-only policy. No notifications, no access logging (cost). `prevent_destroy` + the plan guard block deletion. Encryption is SSE-S3 (docs/contract.md, Storage).

Rollback: n/a — first version.

Test: `terraform test` here (plan mode, offline).

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
| [aws_s3_bucket.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/s3_bucket) | resource |
| [aws_s3_bucket_cors_configuration.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/s3_bucket_cors_configuration) | resource |
| [aws_s3_bucket_lifecycle_configuration.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/s3_bucket_lifecycle_configuration) | resource |
| [aws_s3_bucket_ownership_controls.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/s3_bucket_ownership_controls) | resource |
| [aws_s3_bucket_policy.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/s3_bucket_policy) | resource |
| [aws_s3_bucket_public_access_block.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/s3_bucket_public_access_block) | resource |
| [aws_s3_bucket_server_side_encryption_configuration.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/s3_bucket_server_side_encryption_configuration) | resource |
| [aws_s3_bucket_versioning.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/s3_bucket_versioning) | resource |
| [aws_iam_policy_document.tls_only](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/data-sources/iam_policy_document) | data source |

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_abort_incomplete_multipart_days"></a> [abort\_incomplete\_multipart\_days](#input\_abort\_incomplete\_multipart\_days) | Days after which an incomplete multipart upload is aborted. | `number` | `2` | no |
| <a name="input_cors_allowed_methods"></a> [cors\_allowed\_methods](#input\_cors\_allowed\_methods) | HTTP methods the CORS rule allows. | `list(string)` | <pre>[<br/>  "GET",<br/>  "PUT"<br/>]</pre> | no |
| <a name="input_cors_allowed_origins"></a> [cors\_allowed\_origins](#input\_cors\_allowed\_origins) | Exact browser origins (https://host, no path) allowed to call the bucket through presigned URLs; empty = no CORS configuration. | `list(string)` | `[]` | no |
| <a name="input_expiring_prefixes"></a> [expiring\_prefixes](#input\_expiring\_prefixes) | Prefix (ending in /) mapped to the number of days after which its objects expire, e.g. { "tmp/" = 2 } for unconfirmed uploads. Empty = nothing expires by prefix. | `map(number)` | `{}` | no |
| <a name="input_name"></a> [name](#input\_name) | Globally unique bucket name, e.g. kristoff-prod-docs-<account id> (the account id makes it unique). | `string` | n/a | yes |
| <a name="input_noncurrent_version_expiration_days"></a> [noncurrent\_version\_expiration\_days](#input\_noncurrent\_version\_expiration\_days) | Days after which a noncurrent object version is removed. | `number` | `7` | no |
| <a name="input_tags"></a> [tags](#input\_tags) | Extra tags; the provider's default\_tags carry Project/Stage/Stack/ManagedBy/Repo, the module adds Name. | `map(string)` | `{}` | no |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_bucket_arn"></a> [bucket\_arn](#output\_bucket\_arn) | Bucket ARN (append /* for its objects). |
| <a name="output_bucket_name"></a> [bucket\_name](#output\_bucket\_name) | Bucket name. |
<!-- END_TF_DOCS -->
