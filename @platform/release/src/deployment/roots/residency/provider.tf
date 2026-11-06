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

locals {
  # the regions sealing the machine's keys, both inside the residency
  kms_regions = {
    eu = { primary = "eu-central-1", replica = "eu-west-1" }
    us = { primary = "us-east-1", replica = "us-west-2" }
  }
}

# seal the cloud machine's keys inside its residency
provider "aws" {
  region = local.kms_regions[var.residency].primary
}

provider "aws" {
  alias  = "replica"
  region = local.kms_regions[var.residency].replica
}
