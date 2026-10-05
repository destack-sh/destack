terraform {
  required_version = ">= 1.12.0, < 2.0.0"

  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "5.24.0"
    }
    aws = {
      source  = "hashicorp/aws"
      version = "6.67.0"
    }
  }
}

provider "cloudflare" {}

# sign with the standard credential chain
provider "aws" {
  region = var.aws_region
}

locals {
  account_id = var.account_id
}
