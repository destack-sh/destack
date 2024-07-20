locals {
  aws_global_vpc_network_cidr = "10.0.0.0/16"
  aws_region_by_bench_region = {
    "eu-frankfurt" = "eu-central-1"
  }
  aws_region_availability_zones = {
    "eu-frankfurt" = ["eu-central-1a", "eu-central-1b"]
  }
}

provider "aws" {
  region = local.aws_region_by_bench_region[var.global_region]
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
  count             = length(local.aws_region_availability_zones[var.global_region])
  vpc_id            = aws_vpc.global_vpc.id
  cidr_block        = cidrsubnet(local.aws_global_vpc_network_cidr, 8, count.index)
  availability_zone = local.aws_region_availability_zones[var.global_region][count.index]

  tags = {
    Name = "bench-${var.env}-global-public-subnet-${count.index + 1}"
  }
}
resource "aws_subnet" "global_private" {
  count             = length(local.aws_region_availability_zones[var.global_region])
  vpc_id            = aws_vpc.global_vpc.id
  cidr_block        = cidrsubnet(local.aws_global_vpc_network_cidr, 8, 2 + count.index)
  availability_zone = local.aws_region_availability_zones[var.global_region][count.index]

  tags = {
    Name = "bench-${var.env}-global-private-subnet-${count.index + 1}"
  }
}


# 
# Regional modules
# NOTE :Cleanup: region modules are duplicated because we need them to be legacy modules :StaticRegions
#  (because the kubernetes provider depends on the EKS cluster, and we coan't pass that as an argument without creating a circular dependency)
#

module "region_eu_central_1" {
  source = "./region"

  # general
  bench_version          = file("../version")
  git_commit             = data.external.git.result.sha
  env                    = var.env
  cloud                  = "aws"
  region                 = "eu-frankfurt"
  aws_availability_zones = ["eu-central-1a", "eu-central-1b"]
  global_region          = var.global_region

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
  web_certificate_arn  = aws_acm_certificate.justbench_com.arn
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
    "aws-eu-frankfurt" = module.region_eu_central_1
  }
}

#
# Peering
# 

# peer regional AWS VPCs to the global AWS VPC
resource "aws_vpc_peering_connection" "global_peering" {
  for_each    = local.regions
  vpc_id      = local.regions[each.key].vpc_id
  peer_vpc_id = aws_vpc.global_vpc.id
  peer_region = var.global_region
  tags = {
    Name = "bench-${var.env}-${each.key}-global-peering"
  }
}
resource "aws_vpc_peering_connection_accepter" "global_peering_accepter" {
  for_each                  = local.regions
  vpc_peering_connection_id = aws_vpc_peering_connection.global_peering[each.key].id
  auto_accept               = true
  tags = {
    Name = "bench-${var.env}-${each.key}-global-peering"
  }
}

# peer regional VPCs to each other
locals {
  vpc_ids = {
    for region, mod in local.regions : region => mod.vpc_id
  }
  # generate possible pairs of regions
  region_pairs = [
    for pair in setproduct(keys(local.regions), keys(local.regions)) : pair
    if pair[0] != pair[1]
  ]
  # map into peering connections
  peering_map = {
    for pair in local.region_pairs :
    "${pair[0]}-${pair[1]}" => {
      region1 = pair[0]
      region2 = pair[1]
      vpc1    = local.vpc_ids[pair[0]]
      vpc2    = local.vpc_ids[pair[1]]
    }
  }
}
resource "aws_vpc_peering_connection" "cross_region_peering" {
  for_each    = local.peering_map
  vpc_id      = each.value.vpc1
  peer_vpc_id = each.value.vpc2
  peer_region = local.aws_region_by_bench_region[each.value.region2]
  tags = {
    Name = "bench-${var.env}-${each.value.region1}-${each.value.region2}-cross-region-peering"
  }
}
resource "aws_vpc_peering_connection_accepter" "cross_region_peering_accepter" {
  for_each                  = local.peering_map
  vpc_peering_connection_id = aws_vpc_peering_connection.cross_region_peering[each.key].id
  auto_accept               = true
  tags = {
    Name = "bench-${var.env}-${each.value.region1}-${each.value.region2}-cross-region-peering"
  }
}
