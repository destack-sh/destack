locals {
  aws_region_by_bench_region = {
    "eu-zurich" : "eu-central-2"
    "eu-frankfurt" = "eu-central-1"
  }
}

provider "aws" {
  region = local.aws_region_by_bench_region[var.region]
}

#
# AWS VPC
#

# VPC
resource "aws_vpc" "region_vpc" {
  cidr_block           = var.vpc_network_cidr
  enable_dns_hostnames = true

  tags = {
    Name = "bench-${var.env}-${var.region}-region-vpc"
  }
}

# Subnets
resource "aws_subnet" "public" {
  count             = length(var.aws_availability_zones)
  vpc_id            = aws_vpc.region_vpc.id
  cidr_block        = cidrsubnet(var.vpc_network_cidr, 8, count.index)
  availability_zone = var.aws_availability_zones[count.index]

  tags = {
    Name                     = "bench-${var.env}-${var.region}-public-subnet-${count.index + 1}"
    "kubernetes.io/role/elb" = "1"
  }
}
resource "aws_subnet" "private" {
  count             = length(var.aws_availability_zones)
  vpc_id            = aws_vpc.region_vpc.id
  cidr_block        = cidrsubnet(var.vpc_network_cidr, 8, length(var.aws_availability_zones) + count.index)
  availability_zone = var.aws_availability_zones[count.index]

  tags = {
    Name                              = "bench-${var.env}-${var.region}-private-subnet-${count.index + 1}"
    "kubernetes.io/role/internal-elb" = "1"
  }
}

# Internet Gateway
resource "aws_internet_gateway" "region_vpc" {
  vpc_id = aws_vpc.region_vpc.id

  tags = {
    Name = "bench-${var.env}-${var.region}-internet-gateway"
  }
}

# Elastic IP
resource "aws_eip" "region_vpc" {
  domain     = "vpc"
  depends_on = [aws_internet_gateway.region_vpc]

  tags = {
    Name = "bench-${var.env}-${var.region}-eip"
  }
}

# NAT Gateway
resource "aws_nat_gateway" "region_vpc" {
  allocation_id = aws_eip.region_vpc.id
  subnet_id     = aws_subnet.public[0].id
  depends_on    = [aws_internet_gateway.region_vpc]
}

# Public Route Table
resource "aws_route_table" "public" {
  vpc_id = aws_vpc.region_vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.region_vpc.id
  }

  tags = {
    Name = "bench-${var.env}-${var.region}-public-route-table"
  }
}
resource "aws_route_table_association" "public" {
  count          = length(aws_subnet.public)
  subnet_id      = aws_subnet.public[count.index].id
  route_table_id = aws_route_table.public.id
}

# Private Route Table
resource "aws_route_table" "private" {
  vpc_id = aws_vpc.region_vpc.id

  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.region_vpc.id
  }

  tags = {
    Name = "bench-${var.env}-${var.region}-private-route-table"
  }
}
resource "aws_route_table_association" "private" {
  count          = length(aws_subnet.private)
  subnet_id      = aws_subnet.private[count.index].id
  route_table_id = aws_route_table.private.id
}

#
# AWS EKS cluster
# NOTE :Infra :Architecture: eventually we'll want multiple clusters per region
#

data "aws_caller_identity" "current" {}
module "cluster_0" {
  source = "terraform-aws-modules/eks/aws"

  cluster_name    = "bench-${var.env}-${var.cloud}-${var.region}-cluster-0"
  cluster_version = "1.30"
  iam_role_name   = "bench-${var.env}-${var.region}"
  vpc_id          = aws_vpc.region_vpc.id
  subnet_ids      = aws_subnet.private[*].id

  enable_cluster_creator_admin_permissions = true
  cluster_endpoint_private_access          = true
  cluster_endpoint_public_access           = true

  eks_managed_node_groups = {
    "bench-${var.env}-${var.region}-system-nodes" = {
      instance_types = ["t3.medium"]
      min_size       = 1
      max_size       = 3
      desired_size   = 2

      iam_role_use_name_prefix = false
    }
  }

  cluster_tags = {
    Name = "bench-${var.env}-${var.region}-cluster-0"
  }
}
module "cluster_0_auth" {
  source = "terraform-aws-modules/eks/aws//modules/aws-auth"

  # nocheckin: fix this stupid TF error
  manage_aws_auth_configmap = true
  aws_auth_users = [
    {
      userarn  = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
      username = "root"
      groups   = ["system:masters"]
    },
  ]
}

data "aws_eks_cluster" "cluster_0" {
  name = module.cluster_0.cluster_name
}
data "aws_eks_cluster_auth" "cluster_0" {
  name = module.cluster_0.cluster_name
}
provider "kubernetes" {
  host                   = data.aws_eks_cluster.cluster_0.endpoint
  cluster_ca_certificate = base64decode(data.aws_eks_cluster.cluster_0.certificate_authority.0.data)
  token                  = data.aws_eks_cluster_auth.cluster_0.token
}

# Kubernetes secret for GHCR
resource "kubernetes_secret" "image_pull_secret" {
  metadata {
    name      = "image-pull-secret"
    namespace = "default"
  }

  type = "kubernetes.io/dockerconfigjson"

  data = {
    ".dockerconfigjson" = jsonencode({
      auths = {
        "ghcr.io" = {
          auth = base64encode("${var.ghcr_username}:${var.ghcr_token}")
        }
      }
    })
  }
}

#
# S3 bucket
# 

resource "aws_s3_bucket" "bench_public" {
  bucket = "bench-${var.env}-${var.region}-public"
  tags = {
    Name = "bench-${var.env}-${var.region}-public"
  }
}
