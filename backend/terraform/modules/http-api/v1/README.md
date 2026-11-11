# http-api v1
Since 2026-11-12 · first version (adapted from dexfs http-api v1: JWT authorizer replaced by a Lambda REQUEST authorizer, CORS added)

Purpose: One HTTP API (`$default` stage, auto-deploy) with access logs, stage and per-route throttles, CORS for exact browser origins (exposes `retry-after`), an optional Lambda REQUEST authorizer (simple responses, payload 2.0, no cache, identity source `$request.header.Authorization`), and a Lambda proxy integration per route.

Owns: API (with its CORS configuration), log group, authorizer, integrations, routes, stage. Caller responsibilities: Lambda invoke permissions (`aws_lambda_permission` from `execution_arn`) for every route AND for the authorizer function. No custom domain (the default execute-api endpoint is used).

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
| [aws_apigatewayv2_api.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/apigatewayv2_api) | resource |
| [aws_apigatewayv2_authorizer.lambda](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/apigatewayv2_authorizer) | resource |
| [aws_apigatewayv2_integration.lambda](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/apigatewayv2_integration) | resource |
| [aws_apigatewayv2_route.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/apigatewayv2_route) | resource |
| [aws_apigatewayv2_stage.default](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/apigatewayv2_stage) | resource |
| [aws_cloudwatch_log_group.access](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/cloudwatch_log_group) | resource |

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_access_log_format"></a> [access\_log\_format](#input\_access\_log\_format) | Access-log line format; the default carries no path+query, no headers and no bodies (questions and document titles are personal data). | `string` | `"{\"requestId\":\"$context.requestId\",\"ip\":\"$context.identity.sourceIp\",\"route\":\"$context.routeKey\",\"status\":\"$context.status\",\"latency\":\"$context.responseLatency\",\"authorizerError\":\"$context.authorizer.error\",\"err\":\"$context.integrationErrorMessage\"}"` | no |
| <a name="input_access_log_group_name"></a> [access\_log\_group\_name](#input\_access\_log\_group\_name) | CloudWatch log group for access logs, e.g. /kristoff/prod/api-access. | `string` | n/a | yes |
| <a name="input_access_log_retention_days"></a> [access\_log\_retention\_days](#input\_access\_log\_retention\_days) | Access-log retention in days. | `number` | `30` | no |
| <a name="input_authorizer_invoke_arn"></a> [authorizer\_invoke\_arn](#input\_authorizer\_invoke\_arn) | Invoke ARN of the authorizer function's `live` alias; required when the authorizer is enabled. | `string` | `""` | no |
| <a name="input_cors_allowed_headers"></a> [cors\_allowed\_headers](#input\_cors\_allowed\_headers) | Request headers CORS allows. | `list(string)` | <pre>[<br/>  "authorization",<br/>  "content-type"<br/>]</pre> | no |
| <a name="input_cors_allowed_methods"></a> [cors\_allowed\_methods](#input\_cors\_allowed\_methods) | HTTP methods CORS allows. | `list(string)` | <pre>[<br/>  "GET",<br/>  "POST",<br/>  "PATCH"<br/>]</pre> | no |
| <a name="input_cors_allowed_origins"></a> [cors\_allowed\_origins](#input\_cors\_allowed\_origins) | Exact browser origins allowed by CORS (https://host[:port], or http://localhost:<port>); never *. | `list(string)` | n/a | yes |
| <a name="input_description"></a> [description](#input\_description) | Human description of the API. | `string` | `""` | no |
| <a name="input_lambda_authorizer_enabled"></a> [lambda\_authorizer\_enabled](#input\_lambda\_authorizer\_enabled) | Create the Lambda REQUEST authorizer (simple responses, no cache, identity source the Authorization header). | `bool` | `true` | no |
| <a name="input_name"></a> [name](#input\_name) | API name, e.g. kristoff-prod-api. | `string` | n/a | yes |
| <a name="input_routes"></a> [routes](#input\_routes) | Routes to create: route\_key ("GET /v1/config"), target\_invoke\_arn (the Lambda `live` alias invoke ARN), auth ("lambda" \| "none"), optional per-route throttles. | <pre>list(object({<br/>    route_key              = string<br/>    target_invoke_arn      = string<br/>    auth                   = string<br/>    throttling_rate_limit  = optional(number)<br/>    throttling_burst_limit = optional(number)<br/>  }))</pre> | n/a | yes |
| <a name="input_tags"></a> [tags](#input\_tags) | Extra tags; the module adds Name. | `map(string)` | `{}` | no |
| <a name="input_throttling_burst_limit"></a> [throttling\_burst\_limit](#input\_throttling\_burst\_limit) | Stage default burst. | `number` | n/a | yes |
| <a name="input_throttling_rate_limit"></a> [throttling\_rate\_limit](#input\_throttling\_rate\_limit) | Stage default steady-state requests per second. | `number` | n/a | yes |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_access_log_group_name"></a> [access\_log\_group\_name](#output\_access\_log\_group\_name) | Access-log group name. |
| <a name="output_api_endpoint"></a> [api\_endpoint](#output\_api\_endpoint) | Default execute-api endpoint (https://<id>.execute-api.<region>.amazonaws.com). |
| <a name="output_api_id"></a> [api\_id](#output\_api\_id) | API id. |
| <a name="output_authorizer_id"></a> [authorizer\_id](#output\_authorizer\_id) | Id of the Lambda REQUEST authorizer, or null when it is disabled. |
| <a name="output_execution_arn"></a> [execution\_arn](#output\_execution\_arn) | Execution ARN prefix for aws\_lambda\_permission source\_arn ("<arn>/*/<METHOD>/<path>", "<arn>/authorizers/<id>"). |
| <a name="output_route_keys"></a> [route\_keys](#output\_route\_keys) | Sorted route keys created. |
| <a name="output_stage_name"></a> [stage\_name](#output\_stage\_name) | Stage name ($default). |
<!-- END_TF_DOCS -->
