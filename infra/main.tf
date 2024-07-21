locals {
  bench_version               = file("../version")
  global_region               = "eu-zurich"    # global state
  primary_region              = "eu-frankfurt" # supervisor
  main_website                = "justbench.com"
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


# 
# Regional modules
# NOTE :Cleanup: region modules are duplicated because we need them to be legacy modules :StaticRegions
#  (because the kubernetes provider depends on the EKS cluster, and we coan't pass that as an argument without creating a circular dependency)
#

# static providers for each region for peering
provider "aws" {
  alias  = "eu-zurich"
  region = local.aws_region_by_bench_region["eu-zurich"]
}
provider "aws" {
  alias  = "eu-frankfurt"
  region = local.aws_region_by_bench_region["eu-frankfurt"]
}

# region modules
module "region_aws_eu_frankfurt" {
  source = "./region"

  # general
  bench_version          = local.bench_version
  git_commit             = data.external.git.result.sha
  env                    = var.env
  cloud                  = "aws"
  region                 = "eu-frankfurt"
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
  web_certificate_arn  = aws_acm_certificate.main_website.arn
  cors_allowed_hosts   = var.cors_allowed_hosts
  cors_allowed_origins = var.cors_allowed_origins

  # 3rd party secrets
  sentry_dsn        = var.sentry_dsn
  neon_api_key      = var.neon_api_key
  neon_base_url     = var.neon_base_url
  openai_api_key    = var.openai_api_key
  anthropic_api_key = var.anthropic_api_key
  ghcr_username     = var.ghcr_username
  ghcr_token        = var.ghcr_token
}

# put all regions in a map
locals {
  regions = {
    "aws-eu-frankfurt" = module.region_aws_eu_frankfurt
  }
}

#
# Peering
# NOTE :Cleanup: unfortunately we can't dynamically reference providers, so we have to type out each peering connection
# 

# peer regional AWS VPCs to the global AWS VPC
resource "aws_vpc_peering_connection" "global_peering_eu_frankfurt" {
  provider    = aws.eu-frankfurt
  vpc_id      = module.region_aws_eu_frankfurt.vpc_id
  peer_vpc_id = aws_vpc.global_vpc.id
  peer_region = local.aws_region_by_bench_region["eu-zurich"]
  tags = {
    Name = "bench-${var.env}-global-peering-eu-frankfurt"
  }
}
resource "aws_vpc_peering_connection_accepter" "global_peering_accepter_eu_frankfurt" {
  provider                  = aws.eu-zurich
  vpc_peering_connection_id = aws_vpc_peering_connection.global_peering_eu_frankfurt.id
  auto_accept               = true
  tags = {
    Name = "bench-${var.env}-global-peering-eu-frankfurt-accepter"
  }
}

# peer regional AWS VPCs to each other
# ...
