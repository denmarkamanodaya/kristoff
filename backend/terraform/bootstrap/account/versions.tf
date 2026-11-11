# Bootstrap account module — provider pins.
# Trap: this module is applied by a human once per account with LOCAL state; CI never runs it.

terraform {
  required_version = "~> 1.14"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.67"
    }
  }
}
