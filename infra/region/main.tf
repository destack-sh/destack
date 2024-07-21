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
  enable_dns_hostnames = true
  cidr_block           = var.vpc_network_cidr

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
}

# Elastic IP
resource "aws_eip" "region_vpc" {
  domain     = "vpc"
  depends_on = [aws_internet_gateway.region_vpc]
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
# NOTE :Infra :Architecture: eventually we'll probably have multiple clusters per region
#

resource "aws_eks_cluster" "region_cluster" {
  name     = "bench-${var.env}-${var.cloud}-${var.region}"
  role_arn = aws_iam_role.region_cluster_role.arn

  vpc_config {
    endpoint_private_access = true
    endpoint_public_access  = true
    subnet_ids              = concat(aws_subnet.public[*].id, aws_subnet.private[*].id)
  }
}

# Kubernetes provider
data "aws_eks_cluster" "region_cluster" {
  name = aws_eks_cluster.region_cluster.name
}
data "aws_eks_cluster_auth" "region_cluster" {
  name = aws_eks_cluster.region_cluster.name
}
provider "kubernetes" {
  host                   = data.aws_eks_cluster.region_cluster.endpoint
  cluster_ca_certificate = base64decode(data.aws_eks_cluster.region_cluster.certificate_authority.0.data)
  token                  = data.aws_eks_cluster_auth.region_cluster.token
}

# grant root user full access to the cluster
data "aws_caller_identity" "current" {}
locals {
  config_map_aws_auth = {
    apiVersion = "v1"
    kind       = "ConfigMap"
    metadata = {
      name      = "aws-auth"
      namespace = "kube-system"
    }
    data = {
      mapRoles = yamlencode([
        {
          rolearn  = aws_iam_role.region_node_role.arn
          username = "system:node:{{EC2PrivateDNSName}}"
          groups   = ["system:bootstrappers", "system:nodes"]
        },
      ])
      mapUsers = yamlencode([
        {
          userarn  = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
          username = "admin"
          groups   = ["system:masters"]
        },
      ])
    }
  }
}
resource "null_resource" "aws_auth_config_map_wait" {
  # during initial cluster creation, the aws-auth config map is not immediately available
  depends_on = [aws_eks_cluster.region_cluster]
  provisioner "local-exec" {
    command = "sleep 5"
  }
}
resource "kubernetes_config_map_v1_data" "aws_auth" {
  metadata {
    name      = "aws-auth"
    namespace = "kube-system"
  }

  data = {
    mapRoles = yamlencode(yamldecode(local.config_map_aws_auth.data.mapRoles))
    mapUsers = yamlencode(yamldecode(local.config_map_aws_auth.data.mapUsers))
  }

  force      = true
  depends_on = [aws_eks_cluster.region_cluster, null_resource.aws_auth_config_map_wait]
}

# IAM roles for EKS cluster
resource "aws_iam_role" "region_cluster_role" {
  name = "bench-${var.env}-${var.region}-region-cluster-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "eks.amazonaws.com"
        }
      }
    ]
  })
}
resource "aws_iam_role_policy_attachment" "region_cluster_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSClusterPolicy"
  role       = aws_iam_role.region_cluster_role.name
}
resource "aws_iam_role_policy_attachment" "region_vpc_resource_controller" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSVPCResourceController"
  role       = aws_iam_role.region_cluster_role.name
}

# IAM roles for EKS nodes
resource "aws_iam_role" "region_node_role" {
  name = "bench-${var.env}-${var.region}-region-node-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })
}
resource "aws_iam_role_policy_attachment" "region_worker_node_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
  role       = aws_iam_role.region_node_role.name
}
resource "aws_iam_role_policy_attachment" "region_cni_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  role       = aws_iam_role.region_node_role.name
}
resource "aws_iam_role_policy_attachment" "ec2_container_registry_read_only" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
  role       = aws_iam_role.region_node_role.name
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

