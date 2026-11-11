# lambda-stream-url v1 — plan-mode tests, no AWS calls (dummy credentials, validation skipped).

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
  name                     = "kristoff-prod-ask-stream"
  zip_path                 = "./tests/fixtures/empty.zip"
  role_path                = "/kristoff/lambda/"
  permissions_boundary_arn = "arn:aws:iam::123456789012:policy/kristoff/ci/kristoff-prod-lambda-boundary"
  policy_json              = "{\"Version\":\"2012-10-17\",\"Statement\":[{\"Sid\":\"Table\",\"Effect\":\"Allow\",\"Action\":[\"dynamodb:GetItem\"],\"Resource\":\"arn:aws:dynamodb:ap-southeast-2:123456789012:table/kristoff-prod-core\"}]}"
  cors_allowed_origins     = ["https://kristoff.example.dev"]
}

run "streaming_url_on_the_live_alias" {
  command = plan

  assert {
    condition     = aws_lambda_function_url.this.invoke_mode == "RESPONSE_STREAM" && aws_lambda_function_url.this.authorization_type == "NONE"
    error_message = "the URL streams and carries no IAM auth (the function checks the session token)"
  }
  assert {
    condition     = aws_lambda_function_url.this.qualifier == "live"
    error_message = "the URL serves the live alias"
  }
  assert {
    condition     = module.fn.function_name == "kristoff-prod-ask-stream"
    error_message = "the function is built by lambda-fn v1 with the given name"
  }
}

run "defaults_match_the_contract" {
  command = plan

  assert {
    condition     = output.granted_actions == tolist(["dynamodb:GetItem", "logs:CreateLogStream", "logs:PutLogEvents"])
    error_message = "the role holds the caller's statements plus the logs statement"
  }
}

run "cors_is_post_from_exact_origins" {
  command = plan

  assert {
    condition     = tolist(aws_lambda_function_url.this.cors[0].allow_origins) == tolist(["https://kristoff.example.dev"]) && tolist(aws_lambda_function_url.this.cors[0].allow_methods) == tolist(["POST"])
    error_message = "CORS allows POST from the web origins only"
  }
  assert {
    condition     = toset(aws_lambda_function_url.this.cors[0].allow_headers) == toset(["authorization", "content-type"])
    error_message = "CORS allows the authorization and content-type headers"
  }
  assert {
    condition     = toset(aws_lambda_function_url.this.cors[0].expose_headers) == toset(["retry-after"])
    error_message = "the browser may read Retry-After (contract Clarifications)"
  }
}

run "public_url_needs_both_permissions" {
  command = plan

  assert {
    condition     = aws_lambda_permission.url.action == "lambda:InvokeFunctionUrl" && aws_lambda_permission.url.function_url_auth_type == "NONE" && aws_lambda_permission.url.qualifier == "live"
    error_message = "InvokeFunctionUrl for auth type NONE on the live alias"
  }
  assert {
    condition     = aws_lambda_permission.invoke_via_url.action == "lambda:InvokeFunction" && aws_lambda_permission.invoke_via_url.invoked_via_function_url == true
    error_message = "InvokeFunction is granted only for calls made through the function URL"
  }
}

run "rejects_wildcard_origin" {
  command = plan

  variables {
    cors_allowed_origins = ["*"]
  }

  expect_failures = [var.cors_allowed_origins]
}
