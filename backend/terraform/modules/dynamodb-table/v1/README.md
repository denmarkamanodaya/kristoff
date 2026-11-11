# dynamodb-table v1
Since 2026-11-12 · first version (adapted from dexfs dynamodb-table v1)

Purpose: One on-demand DynamoDB table with optional global secondary indexes and TTL; point-in-time recovery, deletion protection and `prevent_destroy` are always on.

Owns: `aws_dynamodb_table`. A GSI projection cannot change after creation. Key layout is the backend's concern, not the module's.

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
| [aws_dynamodb_table.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/dynamodb_table) | resource |

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_attribute_types"></a> [attribute\_types](#input\_attribute\_types) | Type (S, N or B) of every KEY attribute of the table and its indexes, keyed by attribute name. | `map(string)` | n/a | yes |
| <a name="input_global_secondary_indexes"></a> [global\_secondary\_indexes](#input\_global\_secondary\_indexes) | Global secondary indexes. The projection cannot change after creation; non\_key\_attributes only applies to INCLUDE. | <pre>list(object({<br/>    name               = string<br/>    hash_key           = string<br/>    range_key          = optional(string)<br/>    projection_type    = string<br/>    non_key_attributes = optional(list(string))<br/>  }))</pre> | `[]` | no |
| <a name="input_hash_key"></a> [hash\_key](#input\_hash\_key) | Partition key attribute name (e.g. PK). | `string` | n/a | yes |
| <a name="input_name"></a> [name](#input\_name) | Fully prefixed table name, e.g. kristoff-prod. | `string` | n/a | yes |
| <a name="input_range_key"></a> [range\_key](#input\_range\_key) | Sort key attribute name (e.g. SK); null for a hash-only table. | `string` | `null` | no |
| <a name="input_tags"></a> [tags](#input\_tags) | Extra tags; the provider's default\_tags carry Project/Stage/Stack/ManagedBy/Repo, the module adds Name. | `map(string)` | `{}` | no |
| <a name="input_ttl_attribute"></a> [ttl\_attribute](#input\_ttl\_attribute) | Name of the epoch-seconds attribute DynamoDB expires items by; null disables TTL. | `string` | `"ttl"` | no |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_index_names"></a> [index\_names](#output\_index\_names) | Sorted names of the global secondary indexes. |
| <a name="output_table_arn"></a> [table\_arn](#output\_table\_arn) | Table ARN (append /index/* for its indexes). |
| <a name="output_table_name"></a> [table\_name](#output\_table\_name) | Table name. |
<!-- END_TF_DOCS -->
