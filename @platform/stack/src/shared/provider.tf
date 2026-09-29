terraform {
  required_version = ">= 1.12.0, < 2.0.0"

  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "5.24.0"
    }
  }
}

provider "cloudflare" {}

locals {
  account_id = var.account_id
}
