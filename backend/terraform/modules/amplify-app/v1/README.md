# amplify-app v1
Since 2026-11-12 · first version

Purpose: One Amplify Hosting app (platform WEB, no connected repository) for a static single-page client: one `PRODUCTION` branch, security headers on every path (CSP from the caller, HSTS, nosniff, frame deny, no-referrer, permissions policy, COOP), and the SPA rewrite to `/index.html`. Served only on the Amplify default domain (`https://<branch>.<default domain>`); no custom domain (owner ruling 2026-11-12).

Owns: `aws_amplify_app`, `aws_amplify_branch`. Deployments are uploaded by CI (`aws amplify create-deployment` -> PUT zip -> `start-deployment`); Amplify never builds.

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
| [aws_amplify_app.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/amplify_app) | resource |
| [aws_amplify_branch.this](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/amplify_branch) | resource |

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_branch_name"></a> [branch\_name](#input\_branch\_name) | The one branch deployments go to. | `string` | `"main"` | no |
| <a name="input_content_security_policy"></a> [content\_security\_policy](#input\_content\_security\_policy) | Content-Security-Policy header value sent on every path (the stack builds it from the API, stream and bucket origins). | `string` | n/a | yes |
| <a name="input_description"></a> [description](#input\_description) | Human description of the app. | `string` | `""` | no |
| <a name="input_name"></a> [name](#input\_name) | Amplify app name, e.g. kristoff-prod-web. | `string` | n/a | yes |
| <a name="input_tags"></a> [tags](#input\_tags) | Extra tags; the provider's default\_tags carry Project/Stage/Stack/ManagedBy/Repo, the module adds Name. | `map(string)` | `{}` | no |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_app_id"></a> [app\_id](#output\_app\_id) | Amplify app id (aws amplify create-deployment --app-id). |
| <a name="output_branch_name"></a> [branch\_name](#output\_branch\_name) | Branch the deployments go to. |
| <a name="output_branch_url"></a> [branch\_url](#output\_branch\_url) | HTTPS origin of the branch on the Amplify default domain, https://<branch>.<default domain> (no trailing slash): the web origin. |
| <a name="output_default_domain"></a> [default\_domain](#output\_default\_domain) | Amplify default domain of the app; the branch is served at https://<branch>.<default\_domain>. |
<!-- END_TF_DOCS -->
