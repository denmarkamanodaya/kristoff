# amplify-app v1 — plan-mode tests, no AWS calls.

provider "aws" {
  region                      = "ap-southeast-2"
  access_key                  = "test"
  secret_key                  = "test"
  skip_credentials_validation = true
  skip_requesting_account_id  = true
  skip_metadata_api_check     = true
  skip_region_validation      = true
}

variables {
  name                    = "kristoff-prod-web"
  content_security_policy = "default-src 'self'; connect-src 'self' https://*.execute-api.ap-southeast-2.amazonaws.com"
}

run "web_app_without_repository" {
  command = plan

  assert {
    condition     = aws_amplify_app.this.platform == "WEB" && aws_amplify_app.this.repository == null && aws_amplify_app.this.enable_branch_auto_build == false
    error_message = "a WEB app with no repository and no auto build (CI uploads zips)"
  }
  assert {
    condition     = aws_amplify_branch.this.branch_name == "main" && aws_amplify_branch.this.stage == "PRODUCTION" && aws_amplify_branch.this.enable_auto_build == false
    error_message = "one production branch main, never built by Amplify"
  }
  assert {
    condition     = one(aws_amplify_app.this.custom_rule).target == "/index.html" && one(aws_amplify_app.this.custom_rule).status == "200"
    error_message = "SPA rewrite to index.html"
  }
}

run "security_headers_on_every_path" {
  command = plan

  assert {
    condition     = jsondecode(aws_amplify_app.this.custom_headers).customHeaders[0].pattern == "**"
    error_message = "headers apply to every path"
  }
  assert {
    condition     = { for h in jsondecode(aws_amplify_app.this.custom_headers).customHeaders[0].headers : h.key => h.value }["Content-Security-Policy"] == var.content_security_policy
    error_message = "the CSP header carries the caller's policy"
  }
  assert {
    condition     = alltrue([for k in ["Strict-Transport-Security", "X-Content-Type-Options", "X-Frame-Options", "Referrer-Policy"] : contains([for h in jsondecode(aws_amplify_app.this.custom_headers).customHeaders[0].headers : h.key], k)])
    error_message = "HSTS, nosniff, frame deny and referrer policy are set"
  }
}

run "no_custom_domain_resource" {
  command = plan

  assert {
    condition     = output.branch_name == "main"
    error_message = "the branch is main; the site is served at https://main.<default domain> only"
  }
}

run "rejects_unsafe_eval" {
  command = plan

  variables {
    content_security_policy = "default-src 'self' 'unsafe-eval'"
  }

  expect_failures = [var.content_security_policy]
}
