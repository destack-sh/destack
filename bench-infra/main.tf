terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
    acme = {
      source  = "vancluever/acme"
      version = "~> 2.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }
}


locals {
  version       = file("../version")
  global_region = "eu-zurich" # global state
  main_website  = "heybench.com"

  host_map = {
    "eu-zurich"    = "aws-eu-zurich.host.${local.main_website}:60061/443s"
    "eu-frankfurt" = "aws-eu-frankfurt.host.${local.main_website}:60061/443s"
    "na-virginia"  = "aws-na-virginia.host.${local.main_website}:60061/443s"
  }

  aws_global_vpc_network_cidr = "10.0.0.0/16"
  aws_region_by_bench_region = {
    "eu-zurich" : "eu-central-2"
    "eu-frankfurt" = "eu-central-1"
    "na-virginia"  = "us-east-1"
  }
  aws_region_availability_zones = {
    "eu-zurich"    = ["eu-central-2a", "eu-central-2b"]
    "eu-frankfurt" = ["eu-central-1a", "eu-central-1b"]
    "na-virginia"  = ["us-east-1a", "us-east-1b"]
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
  source = "./region_aws"

  # general
  bench_version          = local.version
  git_commit             = data.external.git.result.sha
  env                    = var.env
  cloud                  = "aws"
  region                 = "eu-frankfurt"
  is_primary             = true
  aws_availability_zones = ["eu-central-1a", "eu-central-1b"]
  host_map               = local.host_map
  supervisor_url         = "https://supervisor.heybench.com:60061"

  # aws
  vpc_network_cidr            = "10.1.0.0/16"
  system_min_cluster_size     = var.system_min_cluster_size
  system_max_cluster_size     = var.system_max_cluster_size
  system_desired_cluster_size = var.system_desired_cluster_size
  system_node_instance_type   = var.system_node_instance_type

  # db
  global_pg_url = "postgresql://${var.global_pg_username}:${random_password.global_pg_password.result}@${aws_rds_cluster.global_pg_primary.endpoint}/${var.global_pg_name}"

  # web
  web_zone_id                     = data.cloudflare_zone.main_website.id
  web_certificate_arn             = aws_acm_certificate.main_website.arn
  web_certificate_pem             = "${acme_certificate.main_website.certificate_pem}${acme_certificate.main_website.issuer_pem}"
  web_certificate_private_key_pem = acme_certificate.main_website.private_key_pem

  # 3rd party secrets
  neon_api_key        = var.neon_api_key
  neon_base_url       = var.neon_base_url
  openai_api_key      = var.openai_api_key
  anthropic_api_key   = var.anthropic_api_key
  openrouter_api_key  = var.openrouter_api_key
  xai_api_key         = var.xai_api_key
  exa_api_key         = var.exa_api_key
  ip_api_key          = var.ip_api_key
  ghcr_username       = var.ghcr_username
  ghcr_token          = var.ghcr_token
  unsplash_access_key = var.unsplash_access_key
  posthog_token       = var.posthog_token
  posthog_host        = var.posthog_host
  grafana_cloud_token = var.grafana_cloud_token
}

# TODO :Infra!: handle multiple supervisor regions/urls? (especially for write access) :MultiRegion
#  (Supervisor only has DB access to global + its own region, so it can't create Benches in other regions!;
#   therefore we have to select supervisor somewhere in the bench-web client?)

# point 'supervisor.<domain>' to the supervisor ingress
resource "cloudflare_record" "supervisor" {
  zone_id         = data.cloudflare_zone.main_website.id
  name            = "supervisor"
  type            = "CNAME"
  content         = module.region_aws_eu_frankfurt.supervisor_hostname
  ttl             = 300
  proxied         = false
  allow_overwrite = true
}
