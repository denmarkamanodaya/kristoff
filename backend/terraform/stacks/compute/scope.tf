# scope_guard — fails the plan when backend/lambda-scope.json does not fit this stack: a function missing or unknown, an
# env key the stack has no value for, or a data action outside the boundary's ceiling (stage-names lambda_*_actions).
# Every function depends on it, so a bad scope file stops the plan before any role is planned.

resource "terraform_data" "scope_guard" {
  input = sha256(jsonencode(local.scope))

  lifecycle {
    precondition {
      condition     = try(sort(keys(local.scope.functions)), []) == sort(local.function_keys)
      error_message = "backend/lambda-scope.json must list exactly the functions ${join(", ", sort(local.function_keys))}; it lists ${join(", ", try(sort(keys(local.scope.functions)), []))}."
    }
    precondition {
      condition     = alltrue([for k, f in try(local.scope.functions, {}) : alltrue([for e in f.env : contains(keys(local.env_values), e)])])
      error_message = "backend/lambda-scope.json names an env key the compute stack has no value for (known: ${join(", ", sort(keys(local.env_values)))})."
    }
    precondition {
      condition     = alltrue([for k, f in try(local.scope.functions, {}) : alltrue([for e in f.ssm : contains(keys(local.ssm_env_arns), e) && contains(f.env, e)])])
      error_message = "backend/lambda-scope.json: every ssm entry must be an SSM_* env key the function also lists in env."
    }
    precondition {
      condition     = alltrue([for k, f in local.fn_scope : alltrue([for a in concat(local.fn_scope[k].table, local.fn_scope[k].gsi1) : contains(module.names.lambda_dynamodb_actions, a)])])
      error_message = "backend/lambda-scope.json: a dynamodb action is outside ${join(", ", module.names.lambda_dynamodb_actions)}."
    }
    precondition {
      condition     = alltrue([for k, f in local.fn_scope : alltrue([for a in local.fn_scope[k].s3_actions : contains(module.names.lambda_s3_actions, a)]) && local.fn_scope[k].s3_prefix == module.names.docs_object_prefix])
      error_message = "backend/lambda-scope.json: an s3 action is outside ${join(", ", module.names.lambda_s3_actions)}, or s3.prefix is not ${module.names.docs_object_prefix}."
    }
  }
}

# The model call must time out before the function does, or a slow answer dies without its error event.
check "anthropic_timeout_fits_the_stream_timeout" {
  assert {
    condition     = var.anthropic_timeout_ms < var.stream_timeout_s * 1000
    error_message = "anthropic_timeout_ms (${var.anthropic_timeout_ms}) must be below the ask-stream timeout (${var.stream_timeout_s} s)."
  }
}
