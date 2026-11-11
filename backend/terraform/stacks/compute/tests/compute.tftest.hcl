# compute stack — plan-mode tests through the real modules, no AWS calls. lambda_zip_dir points at a directory with no
# bundles, so every function uses the placeholder bundle; the scope file is a copy of backend/lambda-scope.json.

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
  stage                = "prod"
  account_id           = "123456789012"
  web_origin           = "https://main.d1abcdef.amplifyapp.com"
  lambda_zip_dir       = "./tests/no-bundles"
  lambda_scope_path    = "./tests/fixtures/lambda-scope.json"
  owner_id             = "kristoff"
  model_ids            = ["claude-sonnet-5-5", "claude-opus-5-5", "claude-haiku-5-5"]
  max_tokens           = 4096
  daily_question_cap   = 100
  anthropic_timeout_ms = 120000
  followup_model_id    = "claude-haiku-5-5"
}

run "twenty_one_functions_with_contract_names_and_sizes" {
  command = plan

  assert {
    condition     = sort(keys(output.function_names)) == sort(["authorizer", "auth-verify", "config-get", "docs-list", "docs-create", "docs-confirm", "docs-rename", "docs-link", "chats-list", "chats-create", "chats-get", "chats-update", "chats-delete", "usage-get", "ask-stream", "projects-list", "projects-create", "projects-rename", "projects-delete", "docs-delete", "docs-split"])
    error_message = "the 21 functions exist: 15 of v1 + chats-delete, plus contract v2's four projects functions, docs-delete and docs-split"
  }
  assert {
    condition     = alltrue([for k, n in output.function_names : n == "kristoff-prod-${k}"])
    error_message = "function names are kristoff-prod-<key>"
  }
  assert {
    condition     = alltrue([for k, z in local.zip_paths : endswith(z, "placeholder/stub.zip")])
    error_message = "without bundles every function deploys the placeholder"
  }
}

run "routes_match_the_contract" {
  command = plan

  assert {
    condition     = sort(module.api.route_keys) == sort(["POST /v1/auth/verify", "GET /v1/config", "GET /v1/docs", "POST /v1/docs", "POST /v1/docs/{id}/confirm", "PATCH /v1/docs/{id}", "GET /v1/docs/{id}/link", "GET /v1/chats", "POST /v1/chats", "GET /v1/chats/{id}", "PATCH /v1/chats/{id}", "DELETE /v1/chats/{id}", "GET /v1/usage", "GET /v1/projects", "POST /v1/projects", "PATCH /v1/projects/{id}", "DELETE /v1/projects/{id}", "DELETE /v1/docs/{id}"])
    error_message = "the API serves exactly 18 routes: 13 of v1 + contract v2's four projects routes and DELETE /v1/docs/{id} (docs-split has no route)"
  }
  assert {
    condition     = { for r in local.routes : r.key => r.auth }["POST /v1/auth/verify"] == "none" && alltrue([for r in local.routes : r.auth == "lambda" if r.key != "POST /v1/auth/verify"])
    error_message = "only auth/verify skips the authorizer"
  }
  assert {
    condition     = length(aws_lambda_permission.api) == 18
    error_message = "one invoke permission per route"
  }
  assert {
    condition     = { for r in local.routes : r.key => r.arn_path }["POST /v1/docs/{id}/confirm"] == "POST/v1/docs/*/confirm"
    error_message = "a path parameter becomes * in the invoke permission"
  }
  assert {
    condition     = aws_lambda_permission.authorizer.principal == "apigateway.amazonaws.com" && aws_lambda_permission.authorizer.qualifier == "live"
    error_message = "API Gateway may invoke the authorizer's live alias"
  }
}

run "least_privilege_from_the_scope_file" {
  command = plan

  assert {
    condition     = output.granted_actions["config-get"] == tolist(["logs:CreateLogStream", "logs:PutLogEvents"])
    error_message = "config-get holds only the logs actions (no data access in its scope)"
  }
  assert {
    condition     = output.granted_actions["authorizer"] == tolist(["logs:CreateLogStream", "logs:PutLogEvents", "ssm:GetParameter"])
    error_message = "the authorizer reads one parameter and nothing else"
  }
  assert {
    condition     = contains(output.granted_actions["docs-create"], "s3:PutObject") && !contains(output.granted_actions["docs-create"], "s3:GetObject")
    error_message = "docs-create may put objects but not read them"
  }
  assert {
    condition     = jsondecode(local.policies["authorizer"]).Statement[0].Resource == ["arn:aws:ssm:ap-southeast-2:123456789012:parameter/kristoff/prod/session-hmac-key"]
    error_message = "the authorizer reads exactly the session key parameter"
  }
  assert {
    condition     = one([for st in jsondecode(local.policies["docs-link"]).Statement : st if st.Sid == "DocObjects"]).Resource == ["arn:aws:s3:::kristoff-prod-docs-123456789012/docs/kristoff/*"]
    error_message = "object actions are scoped to docs/<OWNER_ID>/* of the docs bucket"
  }
  assert {
    condition     = one([for st in jsondecode(local.policies["chats-list"]).Statement : st if st.Sid == "Gsi1Items"]).Resource == ["arn:aws:dynamodb:ap-southeast-2:123456789012:table/kristoff-prod-core/index/GSI1"]
    error_message = "chats-list may Query GSI1"
  }
  assert {
    condition     = length([for st in jsondecode(local.policies["chats-list"]).Statement : st if st.Sid == "TableItems"]) == 0
    error_message = "chats-list has no base-table grant (its scope lists gsi1 only)"
  }
  assert {
    condition     = alltrue([for k, p in local.policies : alltrue([for s in jsondecode(p).Statement : !contains(flatten([s.Resource]), "*") && !anytrue([for a in flatten([s.Action]) : endswith(a, "*")])])])
    error_message = "no wildcard action and no * resource in any function policy"
  }
}

run "env_holds_only_the_listed_keys" {
  command = plan

  assert {
    condition     = sort(keys(local.envs["config-get"])) == sort(["STAGE", "MODEL_IDS", "MAX_ATTACHED_DOCS", "DAILY_QUESTION_CAP"])
    error_message = "a function gets only the env keys its scope lists"
  }
  assert {
    condition     = local.envs["ask-stream"]["SSM_ANTHROPIC_KEY"] == "/kristoff/prod/anthropic-api-key" && local.envs["ask-stream"]["TABLE_NAME"] == "kristoff-prod-core"
    error_message = "env values are parameter NAMES and resource names, never secrets"
  }
}

run "stream_and_api_settings_follow_the_contract" {
  command = plan

  assert {
    condition     = module.ask.function_name == "kristoff-prod-ask-stream"
    error_message = "ask-stream is the streaming function"
  }
}

run "v2_docs_delete_splits_bucket_and_object_actions" {
  command = plan

  assert {
    condition     = one([for st in jsondecode(local.policies["docs-delete"]).Statement : st if st.Sid == "DocObjects"]).Action == ["s3:DeleteObject", "s3:DeleteObjectVersion"] && one([for st in jsondecode(local.policies["docs-delete"]).Statement : st if st.Sid == "DocObjects"]).Resource == ["arn:aws:s3:::kristoff-prod-docs-123456789012/docs/kristoff/*"]
    error_message = "object delete actions only on docs/<OWNER_ID>/*"
  }
  assert {
    condition     = one([for st in jsondecode(local.policies["docs-delete"]).Statement : st if st.Sid == "DocVersionsList"]).Resource == ["arn:aws:s3:::kristoff-prod-docs-123456789012"] && one([for st in jsondecode(local.policies["docs-delete"]).Statement : st if st.Sid == "DocVersionsList"]).Condition.StringLike["s3:prefix"] == ["docs/kristoff/*"]
    error_message = "ListBucketVersions on the bucket ARN, only for the prefix docs/<OWNER_ID>/*"
  }
  assert {
    condition     = module.fn["docs-delete"].function_name == "kristoff-prod-docs-delete"
    error_message = "docs-delete exists"
  }
}

run "v2_docs_split_is_async_with_a_dead_letter_queue" {
  command = plan

  assert {
    condition     = module.fn["docs-split"].async_retry_attempts == 1
    error_message = "docs-split retries a failed async run once, then goes to its DLQ"
  }
  assert {
    condition     = contains(output.granted_actions["docs-split"], "sqs:SendMessage") && !contains(output.granted_actions["docs-confirm"], "sqs:SendMessage")
    error_message = "only docs-split has a DLQ"
  }
  assert {
    condition     = one([for st in jsondecode(local.policies["docs-confirm"]).Statement : st if st.Sid == "InvokeFunctions"]).Resource == ["arn:aws:lambda:ap-southeast-2:123456789012:function:kristoff-prod-docs-split:live"]
    error_message = "docs-confirm may invoke exactly docs-split's live alias"
  }
  assert {
    condition     = local.envs["docs-confirm"]["DOCS_SPLIT_FUNCTION"] == "kristoff-prod-docs-split:live"
    error_message = "DOCS_SPLIT_FUNCTION is the qualified name the grant matches"
  }
  assert {
    condition     = length([for k, p in local.policies : k if anytrue([for st in jsondecode(p).Statement : st.Sid == "InvokeFunctions"])]) == 1
    error_message = "no other function may invoke a function"
  }
  assert {
    condition     = aws_cloudwatch_metric_alarm.docs_split_dlq.threshold == 0 && aws_cloudwatch_metric_alarm.docs_split_dlq.metric_name == "ApproximateNumberOfMessagesVisible"
    error_message = "any message in the docs-split DLQ alarms"
  }
}

run "v2_follow_up_model_reaches_ask_stream" {
  command = plan

  assert {
    condition     = local.envs["ask-stream"]["FOLLOWUP_MODEL_ID"] == "claude-haiku-5-5"
    error_message = "ask-stream gets FOLLOWUP_MODEL_ID"
  }
}

run "rejects_scope_invoking_an_unknown_function" {
  command = plan

  variables {
    lambda_scope_path = "./tests/fixtures/lambda-scope-bad-invoke.json"
  }

  expect_failures = [terraform_data.scope_guard]
}

run "rejects_scope_with_unknown_action" {
  command = plan

  variables {
    lambda_scope_path = "./tests/fixtures/lambda-scope-bad-action.json"
  }

  expect_failures = [terraform_data.scope_guard]
}

run "rejects_scope_missing_a_function" {
  command = plan

  variables {
    lambda_scope_path = "./tests/fixtures/lambda-scope-missing-function.json"
  }

  expect_failures = [terraform_data.scope_guard]
}

run "rejects_scope_with_unknown_env_key" {
  command = plan

  variables {
    lambda_scope_path = "./tests/fixtures/lambda-scope-bad-env.json"
  }

  expect_failures = [terraform_data.scope_guard]
}
