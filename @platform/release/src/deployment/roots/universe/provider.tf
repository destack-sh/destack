terraform {
  required_version = ">= 1.12.0, < 2.0.0"
  required_providers {
    planetscale = {
      source  = "planetscale/planetscale"
      version = "1.11.0"
    }
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
provider "planetscale" {}

provider "cloudflare" {}

# seal the universe's keys in Frankfurt, near its database, replicated to Ireland
provider "aws" {
  region = "eu-central-1"
}

provider "aws" {
  alias  = "replica"
  region = "eu-west-1"
}
