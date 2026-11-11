# The web client on Amplify Hosting: no repository, CI uploads the built zip (.github/workflows/deploy-prod.yml). Served
# only on the Amplify default domain, https://main.<default domain> (owner ruling 2026-11-12: no custom domain); that URL
# is THE web origin every CORS rule admits (output web_origin, read by the compute root).
# The CSP allows the browser to reach only this origin, the HTTP API, the streaming function URL and the docs bucket
# (presigned PUT/GET). The API and URL ids are random and are created by the compute stack after this one, so they are
# admitted by their regional AWS host suffixes; the bucket host is exact.

locals {
  docs_bucket_origin = "https://${module.names.docs_bucket_name}.s3.${var.region}.amazonaws.com"
  content_security_policy = join("; ", [
    "default-src 'self'",
    "script-src 'self'",
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data:",
    "font-src 'self'",
    "connect-src 'self' https://*.execute-api.${var.region}.amazonaws.com https://*.lambda-url.${var.region}.on.aws ${local.docs_bucket_origin}",
    "frame-src ${local.docs_bucket_origin}",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
    "frame-ancestors 'none'",
  ])
}

module "web" {
  source = "../../modules/amplify-app/v1"

  name                    = module.names.amplify_app_name
  description             = "Kristoff ${var.stage} web client"
  content_security_policy = local.content_security_policy
}
