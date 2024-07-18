provider "aws" {
  region = var.global_region
}

#
# Global VPC
#

resource "aws_vpc" "global_vpc" {
  cidr_block           = var.global_vpc_network_cidr
  enable_dns_hostnames = true

  tags = {
    Name = "bench-${var.env}-global-vpc"
  }
}

# Subnets
resource "aws_subnet" "global_public" {
  count             = length(var.region_availability_zones[var.global_region])
  vpc_id            = aws_vpc.global_vpc.id
  cidr_block        = cidrsubnet(var.global_vpc_network_cidr, 8, count.index)
  availability_zone = var.region_availability_zones[var.global_region][count.index]

  tags = {
    Name = "bench-${var.env}-global-public-subnet-${count.index + 1}"
  }
}
resource "aws_subnet" "global_private" {
  count             = length(var.region_availability_zones[var.global_region])
  vpc_id            = aws_vpc.global_vpc.id
  cidr_block        = cidrsubnet(var.global_vpc_network_cidr, 8, 2 + count.index)
  availability_zone = var.region_availability_zones[var.global_region][count.index]

  tags = {
    Name = "bench-${var.env}-global-private-subnet-${count.index + 1}"
  }
}


# 
# Regional modules
# NOTE :Cleanup: region modules are duplicated because we need them to be legacy modules :StaticRegions
#

module "region_eu_central_1" {
  source = "./region"

  # general
  bench_version      = file("../version")
  env                = var.env
  region             = "eu-central-1"
  availability_zones = var.region_availability_zones["eu-central-1"]
  global_region      = var.global_region

  # aws
  vpc_network_cidr            = var.region_vpc_network_cidrs["eu-central-1"]
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

  # api
  cors_allowed_hosts   = var.cors_allowed_hosts
  cors_allowed_origins = var.cors_allowed_origins

  # 3rd party secrets
  sentry_dsn        = var.sentry_dsn
  neon_api_key      = var.neon_api_key
  neon_base_url     = var.neon_base_url
  openai_api_key    = var.openai_api_key
  anthropic_api_key = var.anthropic_api_key
  ghcr_token        = var.ghcr_token
}

# put all regions in a map
locals {
  regions = {
    "eu-central-1" = module.region_eu_central_1
  }
}

# peer regional VPCs to the global VPC
resource "aws_vpc_peering_connection" "global_peering" {
  for_each    = toset(var.regions)
  vpc_id      = local.regions[each.key].vpc_id
  peer_vpc_id = aws_vpc.global_vpc.id
  peer_region = var.global_region
  tags = {
    "Name" = "bench-${var.env}-${each.key}-global-peering"
  }
}
resource "aws_vpc_peering_connection_accepter" "global_peering_accepter" {
  for_each                  = toset(var.regions)
  vpc_peering_connection_id = aws_vpc_peering_connection.global_peering[each.key].id
  auto_accept               = true
}

# peer regional VPCs to each other
locals {
  vpc_ids = {
    for region, mod in local.regions : region => mod.vpc_id
  }
  # generate possible pairs of regions
  region_pairs = [
    for pair in setproduct(var.regions, var.regions) : pair
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
  peer_region = each.value.region2
  tags = {
    "Name" = "bench-${var.env}-${each.value.region1}-${each.value.region2}-cross-region-peering"
  }
}
resource "aws_vpc_peering_connection_accepter" "cross_region_peering_accepter" {
  for_each                  = local.peering_map
  vpc_peering_connection_id = aws_vpc_peering_connection.cross_region_peering[each.key].id
  auto_accept               = true
}