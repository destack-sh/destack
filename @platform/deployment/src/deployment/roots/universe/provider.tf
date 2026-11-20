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

# keep the universe's data and keys in Zurich
provider "aws" {
  region = "eu-central-2"
}
