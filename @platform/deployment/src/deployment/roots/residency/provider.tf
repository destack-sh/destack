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
    random = {
      source  = "hashicorp/random"
      version = "3.9.1"
    }
  }
}

provider "cloudflare" {}

locals {
  # the residency's primary and replica regions
  regions = {
    eu = { primary = "eu-central-1", replica = "eu-west-1" }
    us = { primary = "us-east-1", replica = "us-west-2" }
  }
}

# keep the machine's database and keys in its residency
provider "aws" {
  region = local.regions[var.residency].primary
}

provider "aws" {
  alias  = "replica"
  region = local.regions[var.residency].replica
}
