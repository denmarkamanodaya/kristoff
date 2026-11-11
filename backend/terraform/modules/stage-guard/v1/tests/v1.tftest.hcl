# stage-guard v1 — plan-mode tests. The caller account is overridden; no AWS calls.

provider "aws" {
  region                      = "ap-southeast-2"
  access_key                  = "test"
  secret_key                  = "test"
  skip_credentials_validation = true
  skip_requesting_account_id  = true
  skip_metadata_api_check     = true
  skip_region_validation      = true
}

override_data {
  target = data.aws_caller_identity.current
  values = {
    account_id = "123456789012"
  }
}

variables {
  stack      = "platform"
  stage      = "prod"
  account_id = "123456789012"
  backend_state_json = jsonencode({
    backend = {
      type = "s3"
      config = {
        bucket       = "kristoff-tfstate-123456789012"
        key          = "kristoff/prod/platform.tfstate"
        use_lockfile = true
        encrypt      = true
      }
    }
  })
}

run "passes_when_everything_agrees" {
  command = plan

  assert {
    condition     = output.state_key == "kristoff/prod/platform.tfstate" && output.account_id == "123456789012"
    error_message = "the guard exposes the state key it required"
  }
}

run "fails_on_another_accounts_state_bucket" {
  command = plan

  variables {
    backend_state_json = jsonencode({
      backend = {
        type = "s3"
        config = {
          bucket       = "kristoff-tfstate-999999999999"
          key          = "kristoff/prod/platform.tfstate"
          use_lockfile = true
          encrypt      = true
        }
      }
    })
  }

  expect_failures = [terraform_data.this]
}

run "fails_on_the_other_stacks_state_key" {
  command = plan

  variables {
    backend_state_json = jsonencode({
      backend = {
        type = "s3"
        config = {
          bucket       = "kristoff-tfstate-123456789012"
          key          = "kristoff/prod/compute.tfstate"
          use_lockfile = true
          encrypt      = true
        }
      }
    })
  }

  expect_failures = [terraform_data.this]
}

run "fails_without_an_initialised_backend" {
  command = plan

  variables {
    backend_state_json = ""
  }

  expect_failures = [terraform_data.this]
}

run "fails_without_lockfile" {
  command = plan

  variables {
    backend_state_json = jsonencode({
      backend = {
        type = "s3"
        config = {
          bucket       = "kristoff-tfstate-123456789012"
          key          = "kristoff/prod/platform.tfstate"
          use_lockfile = false
          encrypt      = true
        }
      }
    })
  }

  expect_failures = [terraform_data.this]
}

run "fails_when_credentials_belong_to_another_account" {
  command = plan

  variables {
    account_id = "210987654321"
    backend_state_json = jsonencode({
      backend = {
        type = "s3"
        config = {
          bucket       = "kristoff-tfstate-210987654321"
          key          = "kristoff/prod/platform.tfstate"
          use_lockfile = true
          encrypt      = true
        }
      }
    })
  }

  expect_failures = [terraform_data.this]
}
