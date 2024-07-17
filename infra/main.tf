provider "aws" {
  region = var.aws_region
}

#
# AWS VPC
#

# VPC
resource "aws_vpc" "eks_vpc" {
  enable_dns_hostnames = true
  cidr_block           = var.vpc_network_cidr

  tags = {
    Name = "eks-vpc"
  }
}

# Subnets
resource "aws_subnet" "public" {
  count             = 2
  vpc_id            = aws_vpc.eks_vpc.id
  cidr_block        = cidrsubnet(var.vpc_network_cidr, 8, count.index)
  availability_zone = var.aws_availability_zones[count.index]

  tags = {
    Name                     = "Public Subnet ${count.index + 1}"
    "kubernetes.io/role/elb" = "1"
  }
}

resource "aws_subnet" "private" {
  count             = 2
  vpc_id            = aws_vpc.eks_vpc.id
  cidr_block        = cidrsubnet(var.vpc_network_cidr, 8, count.index + 2)
  availability_zone = var.aws_availability_zones[count.index]

  tags = {
    Name                              = "Private Subnet ${count.index + 1}"
    "kubernetes.io/role/internal-elb" = "1"
  }
}

#
# AWS EKS cluster
#

resource "aws_eks_cluster" "eks_cluster" {
  name     = "bench-${var.env}"
  role_arn = aws_iam_role.eks_cluster_role.arn

  vpc_config {
    subnet_ids = concat(aws_subnet.public[*].id, aws_subnet.private[*].id)
  }
}

data "aws_eks_cluster" "eks_cluster" {
  name = aws_eks_cluster.eks_cluster.name
}

data "aws_eks_cluster_auth" "eks_cluster" {
  name = aws_eks_cluster.eks_cluster.name
}


provider "kubernetes" {
  host                   = data.aws_eks_cluster.eks_cluster.endpoint
  cluster_ca_certificate = base64decode(data.aws_eks_cluster.eks_cluster.certificate_authority[0].data)
  token                  = data.aws_eks_cluster_auth.eks_cluster.token
}

# IAM roles for EKS
resource "aws_iam_role" "eks_cluster_role" {
  name = "${var.env}-eks-cluster-role"

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

resource "aws_iam_role_policy_attachment" "eks_cluster_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSClusterPolicy"
  role       = aws_iam_role.eks_cluster_role.name
}

resource "aws_iam_role_policy_attachment" "eks_vpc_resource_controller" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSVPCResourceController"
  role       = aws_iam_role.eks_cluster_role.name
}

# EKS Node Role
resource "aws_iam_role" "eks_node_role" {
  name = "${var.env}-eks-node-role"

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

resource "aws_iam_role_policy_attachment" "eks_worker_node_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
  role       = aws_iam_role.eks_node_role.name
}

resource "aws_iam_role_policy_attachment" "eks_cni_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  role       = aws_iam_role.eks_node_role.name
}

resource "aws_iam_role_policy_attachment" "ec2_container_registry_read_only" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
  role       = aws_iam_role.eks_node_role.name
}

# system nodes
resource "aws_eks_node_group" "eks_system_nodes" {
  cluster_name    = aws_eks_cluster.eks_cluster.name
  node_group_name = "${var.env}-eks-system"
  node_role_arn   = aws_iam_role.eks_node_role.arn
  subnet_ids      = aws_subnet.private[*].id

  scaling_config {
    desired_size = var.system_desired_cluster_size
    max_size     = var.system_max_cluster_size
    min_size     = var.system_min_cluster_size
  }

  instance_types = var.system_node_instance_types
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
          auth = base64encode(var.ghcr_token)
        }
      }
    })
  }
}

#
# AWS RDS Aurora 
# 

resource "aws_security_group" "rds_sg" {
  name        = "${var.env}-rds-sg"
  description = "Allow inbound traffic to the RDS cluster"
  vpc_id      = aws_vpc.eks_vpc.id

  ingress {
    description = "Allow inbound traffic to the RDS cluster"
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = [var.vpc_network_cidr]
  }
}

resource "aws_rds_cluster" "global_pg" {
  cluster_identifier      = "${var.env}-global-db"
  engine                  = "aurora-postgresql"
  engine_mode             = "provisioned"
  engine_version          = "16.2"
  database_name           = "postgres"
  master_username         = "postgres"
  master_password         = var.global_pg_password
  backup_retention_period = 7
  preferred_backup_window = "06:00-08:00"

  vpc_security_group_ids = [aws_security_group.rds_sg.id]
}

resource "aws_rds_cluster_instance" "global_pg_instance" {
  count              = 1
  identifier         = "${var.env}-global-db-${count.index}"
  cluster_identifier = aws_rds_cluster.global_pg.id
  instance_class     = "db.t3.medium"
  engine             = aws_rds_cluster.global_pg.engine
  engine_version     = aws_rds_cluster.global_pg.engine_version
}
