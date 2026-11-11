# amplify-app v1 — one Amplify Hosting app for a static single-page web client WITHOUT a connected repository: CI builds
# the site and uploads a zip (create-deployment -> PUT -> start-deployment). One branch, security headers on every path,
# and an SPA rewrite. Served on the Amplify default domain only (https://<branch>.<default domain>); no custom domain.
# Trap: with no repository, Amplify never builds (enable_branch_auto_build stays false).

locals {
  # Amplify custom headers are a YAML document (customHeaders -> pattern -> headers). jsonencode output is valid YAML.
  custom_headers = jsonencode({
    customHeaders = [{
      pattern = "**"
      headers = [
        { key = "Content-Security-Policy", value = var.content_security_policy },
        { key = "Strict-Transport-Security", value = "max-age=31536000; includeSubDomains" },
        { key = "X-Content-Type-Options", value = "nosniff" },
        { key = "X-Frame-Options", value = "DENY" },
        { key = "Referrer-Policy", value = "no-referrer" },
        { key = "Permissions-Policy", value = "camera=(), microphone=(), geolocation=(), payment=()" },
        { key = "Cross-Origin-Opener-Policy", value = "same-origin" },
      ]
    }]
  })
}

resource "aws_amplify_app" "this" {
  name                     = var.name
  description              = var.description == "" ? null : var.description
  platform                 = "WEB"
  enable_branch_auto_build = false
  custom_headers           = local.custom_headers

  # Single-page app: every path without a file extension serves index.html (Amplify's documented SPA rewrite).
  custom_rule {
    source = "</^[^.]+$|\\.(?!(css|gif|ico|jpg|jpeg|js|json|png|svg|txt|webp|woff|woff2|map|webmanifest)$)([^.]+$)/>"
    target = "/index.html"
    status = "200"
  }

  tags = merge(var.tags, { Name = var.name })
}

resource "aws_amplify_branch" "this" {
  app_id            = aws_amplify_app.this.id
  branch_name       = var.branch_name
  stage             = "PRODUCTION"
  enable_auto_build = false

  tags = merge(var.tags, { Name = "${var.name}-${var.branch_name}" })
}
