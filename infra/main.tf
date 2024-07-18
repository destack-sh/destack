provider "aws" {
  region = var.global_region
}

provider "aws" {
  alias  = "eks"
  region = var.global_region
}

provider "kubernetes" {
  alias       = "eks"
  config_path = "~/.kube/config"
}

#
# Global VPC
#

resource "aws_vpc" "global_vpc" {
  cidr_block           = var.global_vpc_network_cidr
  enable_dns_hostnames = true

  tags = {
    Name = "${var.env}-global-vpc"
  }
}

# Subnets
resource "aws_subnet" "global_public" {
  count             = length(var.global_availability_zones)
  vpc_id            = aws_vpc.global_vpc.id
  cidr_block        = cidrsubnet(var.global_vpc_network_cidr, 8, count.index)
  availability_zone = var.global_availability_zones[count.index]

  tags = {
    Name = "${var.env}-global-public-subnet-${count.index + 1}"
  }
}
resource "aws_subnet" "global_private" {
  count             = length(var.global_availability_zones)
  vpc_id            = aws_vpc.global_vpc.id
  cidr_block        = cidrsubnet(var.global_vpc_network_cidr, 8, 2 + count.index)
  availability_zone = var.global_availability_zones[count.index]

  tags = {
    Name = "${var.env}-global-private-subnet-${count.index + 1}"
  }
}


# 
# Regional modules
#


module "regions" {
  source   = "./region"
  for_each = toset(var.regions)

  # local providers
  providers = {
    aws        = aws.eks
    kubernetes = kubernetes.eks
  }

  # general
  env                = var.env
  region             = each.key
  availability_zones = var.region_availability_zones[each.key]
  global_region      = var.global_region
  is_global_region   = each.key == var.global_region

  # aws
  vpc_network_cidr            = var.region_vpc_network_cidrs[each.key]
  system_min_cluster_size     = var.system_min_cluster_size
  system_max_cluster_size     = var.system_max_cluster_size
  system_desired_cluster_size = var.system_desired_cluster_size
  system_node_instance_type   = var.system_node_instance_type

  # db
  global_pg_name       = var.global_pg_name
  global_pg_username   = var.global_pg_username
  global_pg_password   = var.global_pg_password
  global_pg_crypto_key = var.global_pg_crypto_key

  # api
  cors_allowed_hosts   = var.cors_allowed_hosts
  cors_allowed_origins = var.cors_allowed_origins
  webapp_url           = var.webapp_url

  # 3rd party secrets
  neon_api_key      = var.neon_api_key
  neon_base_url     = var.neon_base_url
  openai_api_key    = var.openai_api_key
  anthropic_api_key = var.anthropic_api_key
  ghcr_token        = var.ghcr_token
}

# peer regional VPCs to the global VPC
resource "aws_vpc_peering_connection" "global_peering" {
  for_each    = toset(var.regions)
  vpc_id      = module.regions[each.key].vpc_id
  peer_vpc_id = aws_vpc.global_vpc.id
  peer_region = var.global_region
  auto_accept = true

  tags = {
    "Name" = "${var.env}-${each.key}-global-peering" 
  }
}
# peer regional VPCs to each other
locals {
  vpc_ids = {
    for region, mod in module.regions : region => mod.vpc_id
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
  auto_accept = true

  tags = {
    "Name" = "${var.env}-${each.value.region1}-${each.value.region2}-cross-region-peering"
  }
}


#
# Global RDS Aurora 
# 

resource "aws_security_group" "global_pg_security_group" {
  name        = "${var.env}-global-pg-security-group"
  description = "Allow inbound traffic to the global RDS cluster"
  vpc_id      = aws_vpc.global_vpc.id

  ingress {
    description = "Allow inbound traffic to the global RDS cluster"
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/16"]
  }
}

resource "aws_db_subnet_group" "global_pg_subnet_group" {
  name       = "${var.env}-global-pg-subnet-group"
  subnet_ids = aws_subnet.global_private[*].id

  tags = {
    Name = "${var.env}-rds-subnet-group"
  }
}

resource "aws_rds_global_cluster" "global_pg" {
  global_cluster_identifier = "${var.env}-global-pg"
  engine                    = "aurora-postgresql"
  engine_version            = "16.2"
  database_name             = var.global_pg_name
}

resource "aws_rds_cluster" "global_pg_primary" {
  cluster_identifier        = aws_rds_global_cluster.global_pg.id
  engine                    = aws_rds_global_cluster.global_pg.engine
  engine_version            = aws_rds_global_cluster.global_pg.engine_version
  database_name             = var.global_pg_name
  master_username           = var.global_pg_username
  master_password           = var.global_pg_password
  backup_retention_period   = 7
  preferred_backup_window   = "06:00-08:00"
  storage_encrypted         = true
  final_snapshot_identifier = "${var.env}-global-pg-final-snapshot"

  vpc_security_group_ids         = [aws_security_group.global_pg_security_group.id]
  db_subnet_group_name           = aws_db_subnet_group.global_pg_subnet_group.name
  enable_local_write_forwarding  = true
  enable_global_write_forwarding = true
}

resource "aws_rds_cluster_instance" "global_pg_primary_instance" {
  count                      = 1
  identifier                 = "${var.env}-global-db-${count.index}"
  cluster_identifier         = aws_rds_cluster.global_pg_primary.id
  instance_class             = "db.t3.medium"
  engine                     = aws_rds_cluster.global_pg_primary.engine
  engine_version             = aws_rds_cluster.global_pg_primary.engine_version
  auto_minor_version_upgrade = true
  availability_zone          = var.global_availability_zones[count.index]
}
