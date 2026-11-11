# amplify-app v1 — Terraform and provider pins (one constraint everywhere; .terraform.lock.hcl pins the exact provider).

terraform {
  required_version = "~> 1.14"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.67"
    }
  }
}
