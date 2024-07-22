terraform {
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
    acme = {
      source  = "vancluever/acme"
      version = "~> 2.0"
    }
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}


locals {
  bench_version = file("../version")
  global_region = "eu-zurich" # global state
  regions       = ["eu-frankfurt"]
  main_website  = "justbench.com"

  host_map = {
    "eu-frankfurt" = "aws-eu-frankfurt.host.${local.main_website}"
  }

  aws_global_vpc_network_cidr = "10.0.0.0/16"
  aws_region_by_bench_region = {
    "eu-zurich" : "eu-central-2"
    "eu-frankfurt" = "eu-central-1"
  }
  aws_region_availability_zones = {
    "eu-zurich"    = ["eu-central-2a", "eu-central-2b"]
    "eu-frankfurt" = ["eu-central-1a", "eu-central-1b"]
  }
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}
data "cloudflare_zone" "main_website" {
  name = local.main_website
}

provider "aws" {
  region = local.aws_region_by_bench_region[local.global_region]
}

data "external" "git" {
  program = [
    "git",
    "log",
    "--pretty=format:{\"sha\": \"%h\"}",
    "-1",
    "HEAD"
  ]
}

#
# Global AWS VPC
#

# VPC
resource "aws_vpc" "global_vpc" {
  cidr_block           = local.aws_global_vpc_network_cidr
  enable_dns_hostnames = true

  tags = {
    Name = "bench-${var.env}-global-vpc"
  }
}

# global subnets
resource "aws_subnet" "global_public" {
  count             = length(local.aws_region_availability_zones[local.global_region])
  vpc_id            = aws_vpc.global_vpc.id
  cidr_block        = cidrsubnet(local.aws_global_vpc_network_cidr, 8, count.index)
  availability_zone = local.aws_region_availability_zones[local.global_region][count.index]

  tags = {
    Name = "bench-${var.env}-global-public-subnet-${count.index + 1}"
  }
}
resource "aws_subnet" "global_private" {
  count             = length(local.aws_region_availability_zones[local.global_region])
  vpc_id            = aws_vpc.global_vpc.id
  cidr_block        = cidrsubnet(local.aws_global_vpc_network_cidr, 8, 2 + count.index)
  availability_zone = local.aws_region_availability_zones[local.global_region][count.index]

  tags = {
    Name = "bench-${var.env}-global-private-subnet-${count.index + 1}"
  }
}

# Internet Gateway
resource "aws_internet_gateway" "global_vpc" {
  vpc_id = aws_vpc.global_vpc.id

  tags = {
    Name = "bench-${var.env}-global-internet-gateway"
  }
}

# Public Route Table
resource "aws_route_table" "global_public" {
  vpc_id = aws_vpc.global_vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.global_vpc.id
  }

  tags = {
    Name = "bench-${var.env}-global-public-route-table"
  }
}
resource "aws_route_table_association" "global_public" {
  count          = length(aws_subnet.global_public)
  subnet_id      = aws_subnet.global_public[count.index].id
  route_table_id = aws_route_table.global_public.id
}

# 
# Regions
# NOTE :Cleanup: region modules are typed out manually because they're legacy modules :StaticRegions
#  (because the kubernetes provider inside needs the EKS cluster inside,
#   and we can't pass them as arguments without creating a circular dependency (?))
#

# region modules
module "region_aws_eu_frankfurt" {
  source = "./region"

  # general
  bench_version          = local.bench_version
  git_commit             = data.external.git.result.sha
  env                    = var.env
  cloud                  = "aws"
  region                 = "eu-frankfurt"
  is_primary             = true
  host_map               = local.host_map
  aws_availability_zones = ["eu-central-1a", "eu-central-1b"]

  # aws
  vpc_network_cidr            = "10.1.0.0/16"
  system_min_cluster_size     = var.system_min_cluster_size
  system_max_cluster_size     = var.system_max_cluster_size
  system_desired_cluster_size = var.system_desired_cluster_size
  system_node_instance_type   = var.system_node_instance_type

  # db
  global_pg_host       = aws_rds_cluster.global_pg_primary.endpoint
  global_pg_name       = var.global_pg_name
  global_pg_username   = var.global_pg_username
  global_pg_password   = var.global_pg_password
  global_pg_crypto_key = var.global_pg_crypto_key

  # web
  web_certificate_arn            = aws_acm_certificate.main_website.arn
  web_certificate_pem             = acme_certificate.main_website.certificate_pem
  web_certificate_private_key_pem = acme_certificate.main_website.private_key_pem

  # 3rd party secrets
  sentry_dsn        = var.sentry_dsn
  neon_api_key      = var.neon_api_key
  neon_base_url     = var.neon_base_url
  openai_api_key    = var.openai_api_key
  anthropic_api_key = var.anthropic_api_key
  ghcr_username     = var.ghcr_username
  ghcr_token        = var.ghcr_token
}
