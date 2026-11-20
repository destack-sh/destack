terraform {
  required_version = ">= 1.12.0, < 2.0.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "6.67.0"
    }
  }
}

# manage the platform account's identities as its administrator
provider "aws" {
  region  = var.aws_region
  profile = var.platform_profile
}

# manage Identity Center in the organization's management account
provider "aws" {
  alias   = "management"
  region  = var.identity_center_region
  profile = var.management_profile
}
