# platform stack — plan-mode tests through the real modules, no AWS calls.

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
  stage       = "prod"
  account_id  = "123456789012"
  alert_email = "owner@example.com"
}

# The Amplify default domain is only known after the apply; pin it so the web origin can be asserted.
override_resource {
  target          = module.web.aws_amplify_app.this
  override_during = plan
  values = {
    default_domain = "d1abcdef.amplifyapp.com"
  }
}

run "web_origin_is_the_amplify_branch_url" {
  command = plan

  assert {
    condition     = output.web_origin == "https://main.d1abcdef.amplifyapp.com"
    error_message = "the one web origin is https://main.<default domain>"
  }
}

run "names_come_from_the_contract" {
  command = plan

  assert {
    condition     = module.table.table_name == "kristoff-prod-core" && module.docs_bucket.bucket_name == "kristoff-prod-docs-123456789012"
    error_message = "table and bucket names are the contract's"
  }
  assert {
    condition     = aws_sns_topic.alerts.name == "kristoff-prod-alerts"
    error_message = "the alerts topic carries the stage prefix"
  }
}

run "table_has_gsi1_and_no_ttl" {
  command = plan

  assert {
    condition     = module.table.index_names == tolist(["GSI1"])
    error_message = "the one index is GSI1"
  }
}

run "kill_switch_is_created_on_and_owner_flipped" {
  command = plan

  assert {
    condition     = aws_ssm_parameter.ai_enabled.name == "/kristoff/prod/ai-enabled" && aws_ssm_parameter.ai_enabled.type == "String" && aws_ssm_parameter.ai_enabled.value == "true"
    error_message = "ai-enabled is a String created as true"
  }
}

run "secrets_are_never_terraform_resources" {
  command = plan

  assert {
    condition     = jsonencode(output.secret_param_names) == jsonencode(["/kristoff/prod/anthropic-api-key", "/kristoff/prod/totp-secret", "/kristoff/prod/session-hmac-key"])
    error_message = "the three SecureString names are exposed for the runbook"
  }
}

run "web_csp_admits_only_the_backends" {
  command = plan

  assert {
    condition     = strcontains(local.content_security_policy, "connect-src 'self' https://*.execute-api.ap-southeast-2.amazonaws.com https://*.lambda-url.ap-southeast-2.on.aws https://kristoff-prod-docs-123456789012.s3.ap-southeast-2.amazonaws.com")
    error_message = "connect-src is the API, the function URL and the docs bucket"
  }
  assert {
    condition     = strcontains(local.content_security_policy, "frame-ancestors 'none'") && strcontains(local.content_security_policy, "script-src 'self'")
    error_message = "no framing, scripts from self only"
  }
}

run "rejects_a_non_email_alert_address" {
  command = plan

  variables {
    alert_email = "not-an-address"
  }

  expect_failures = [var.alert_email]
}
