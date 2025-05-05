terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.0"
    }
    helm = {
      source  = "hashicorp/helm"
      version = ">= 2.14.0"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }
}

locals {
  prefix         = "bench-${var.env}-${var.cloud}-${var.region}"
  otlp_endpoint  = "http://otel-collector.monitoring.svc.cluster.local:4317"
  supervisor_url = "http://${local.prefix}-supervisor.default.svc.cluster.local:60061"
  aws_region_by_bench_region = {
    "eu-zurich"    = "eu-central-2"
    "eu-frankfurt" = "eu-central-1"
    "us-east-1"    = "us-east-1"
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
# NOTE :Infra :Architecture: right now 1 region = 1 cluster, but of course we'll later want multiple clusters per region
#

data "aws_caller_identity" "current" {}
module "cluster_0" {
  source = "terraform-aws-modules/eks/aws"

  cluster_name    = "bench-${var.env}-${var.cloud}-${var.region}-0"
  cluster_version = "1.32"
  iam_role_name   = "bench-${var.env}-${var.region}"
  vpc_id          = aws_vpc.region_vpc.id
  subnet_ids      = aws_subnet.private[*].id

  enable_cluster_creator_admin_permissions = true
  cluster_endpoint_private_access          = true
  cluster_endpoint_public_access           = true

  eks_managed_node_groups = {
    "bench-${var.env}-${var.cloud}-${var.region}-system" = {
      instance_types = ["t4g.large"]
      ami_type       = "AL2_ARM_64"
      min_size       = 2
      max_size       = 6
      desired_size   = 3

      iam_role_use_name_prefix = false
    }
  }

  cluster_tags = {
    Name = "bench-${var.env}-${var.region}-0"
  }
}
module "cluster_0_auth" {
  source = "github.com/terraform-aws-modules/terraform-aws-eks//modules/aws-auth"

  manage_aws_auth_configmap = true
  aws_auth_users = [
    {
      userarn  = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
      username = "root"
      groups   = ["system:masters"]
    },
  ]
}

# Kubernetes/Helm provider
data "aws_eks_cluster" "cluster_0" {
  name       = module.cluster_0.cluster_name
  depends_on = [module.cluster_0]
}
data "aws_eks_cluster_auth" "cluster_0" {
  name       = module.cluster_0.cluster_name
  depends_on = [module.cluster_0]
}
provider "kubernetes" {
  host                   = data.aws_eks_cluster.cluster_0.endpoint
  cluster_ca_certificate = base64decode(data.aws_eks_cluster.cluster_0.certificate_authority.0.data)
  token                  = data.aws_eks_cluster_auth.cluster_0.token
}
provider "helm" {
  kubernetes {
    host                   = data.aws_eks_cluster.cluster_0.endpoint
    cluster_ca_certificate = base64decode(data.aws_eks_cluster.cluster_0.certificate_authority[0].data)
    token                  = data.aws_eks_cluster_auth.cluster_0.token
  }
}

# image pull secret (GHCR)
resource "kubernetes_secret" "image_pull_secret" {
  metadata {
    name      = "bench-${var.env}-${var.cloud}-${var.region}-image-pull-secret"
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

# cert secret (LE)
resource "kubernetes_secret" "web_certificate_secret" {
  metadata {
    name      = "bench-${var.env}-${var.cloud}-${var.region}-cert-secret"
    namespace = "default"
  }

  type = "kubernetes.io/tls"

  data = {
    "tls.crt" = var.web_certificate_pem
    "tls.key" = var.web_certificate_private_key_pem
  }
}

#
# S3 bucket
# 

resource "aws_s3_bucket" "bench_files" {
  bucket = "bench-${var.env}-${var.cloud}-${var.region}-files"
  tags = {
    Name = "bench-${var.env}-${var.cloud}-${var.region}-files"
  }
}
# CORS (allow all)
resource "aws_s3_bucket_cors_configuration" "bench_files_cors" {
  bucket = aws_s3_bucket.bench_files.id

  cors_rule {
    allowed_headers = ["*"]
    allowed_methods = ["GET", "HEAD", "PUT", "POST", "DELETE"]
    allowed_origins = ["*"]
    expose_headers  = ["ETag"]
    max_age_seconds = 3000
  }
}
