#
# Providers
# 

provider "aws" {
  region = var.aws_region
}

provider "kubernetes" {
  host                   = aws_eks_cluster.eks_cluster.endpoint
  cluster_ca_certificate = base64decode(aws_eks_cluster.eks_cluster.certificate_authority[0].data)
  token                  = data.aws_eks_cluster.eks_cluster.name.token
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
  availability_zone = data.aws_availability_zones.available.names[count.index]

  tags = {
    Name                     = "Public Subnet ${count.index + 1}"
    "kubernetes.io/role/elb" = "1"
  }
}

resource "aws_subnet" "private" {
  count             = 2
  vpc_id            = aws_vpc.eks_vpc.id
  cidr_block        = cidrsubnet(var.vpc_network_cidr, 8, count.index + 2)
  availability_zone = data.aws_availability_zones.available.names[count.index]

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

resource "aws_eks_node_group" "eks_nodes" {
  cluster_name    = aws_eks_cluster.eks_cluster.name
  node_group_name = "eks-nodes"
  node_role_arn   = aws_iam_role.eks_node_role.arn
  subnet_ids      = aws_subnet.private[*].id

  scaling_config {
    desired_size = var.desired_cluster_size
    max_size     = var.max_cluster_size
    min_size     = var.min_cluster_size
  }

  instance_types = [var.eks_node_instance_type]
}

# IAM roles for EKS
resource "aws_iam_role" "eks_cluster_role" {
  name = "eks-cluster-role"
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
resource "aws_iam_role" "eks_node_role" {
  name = "eks-node-role"
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

# k8 secret for GHCR
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
  db_subnet_group_name   = aws_db_subnet_group.rds_subnet_group.name
}

resource "aws_rds_cluster_instance" "global_pg_instance" {
  count              = 1
  identifier         = "${var.env}-global-db-${count.index}"
  cluster_identifier = aws_rds_cluster.global_pg.id
  instance_class     = "db.t3.medium"
  engine             = aws_rds_cluster.global_pg.engine
  engine_version     = aws_rds_cluster.global_pg.engine_version
}

# 
# System (Supervisor/Host)
# NOTE :Infra! :Robustness: deploy Supervisor and Host separately
# 

# envoy
resource "kubernetes_config_map" "envoy_config" {
  metadata {
    name      = "envoy-config"
    namespace = "default"
  }

  data = {
    "envoy.yaml" = file("envoy.yaml")
  }
}

# System deployment
resource "kubernetes_deployment" "system" {
  metadata {
    name      = "system"
    namespace = "default"
    labels = {
      app = "system"
    }
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "system"
      }
    }

    template {
      metadata {
        labels = {
          app = "system"
        }
        annotations = {
          "prometheus.io/scrape" = "true"
        }
      }

      spec {
        init_container {
          name    = "system-migrate"
          image   = "ghcr.io/symbolx/bench-system:${var.version}"
          command = ["/bin/sh", "-c"]
          args    = ["python bench.py migrate apply"]

          env {
            name  = "ENVIRONMENT"
            value = var.env
          }
          # Add other environment variables as needed
        }

        container {
          name  = "envoy"
          image = "envoyproxy/envoy:v1.28-latest"
          port {
            container_port = 8080
            name           = "grpc-web"
          }
          volume_mount {
            name       = "envoy-config"
            mount_path = "/etc/envoy"
            read_only  = true
          }
        }

        container {
          name  = "system"
          image = "ghcr.io/symbolx/bench-system:${var.version}"

          port {
            container_port = 80
            name           = "http"
          }
          port {
            container_port = 50051
            name           = "grpc"
          }

          env {
            name  = "ENVIRONMENT"
            value = var.env
          }
          env {
            name  = "REGION"
            value = var.aws_region
          }

          env {
            name  = "GLOBAL_PG_HOST"
            value = aws_rds_cluster.global_pg.endpoint
          }
          env {
            name  = "GLOBAL_PG_PASSWORD"
            value = var.global_pg_password
          }
          env {
            name  = "GLOBAL_PG_CRYPTO_KEY"
            value = var.global_pg_crypto_key
          }

          env {
            name  = "CORS_ALLOWED_HOSTS"
            value = var.cors_allowed_hosts
          }
          env {
            name  = "CORS_ALLOWED_ORIGINS"
            value = var.cors_allowed_origins
          }

          env {
            name  = "SENTRY_DSN"
            value = var.sentry_dsn
          }
          env {
            name  = "OPENAI_API_KEY"
            value = var.openai_api_key
          }
          env {
            name  = "ANTHROPIC_API_KEY"
            value = var.anthropic_api_key
          }

          command = ["python", "bench.py", "serve", "system"]

          resources {
            requests = {
              cpu    = "2000m"
              memory = "2000Mi"
            }
          }
        }

        volume {
          name = "envoy-config"
          config_map {
            name = kubernetes_config_map.envoy_config.metadata[0].name
          }
        }

        image_pull_secrets {
          name = kubernetes_secret.image_pull_secret.metadata[0].name
        }

        service_account_name = kubernetes_service_account.system_service_account.metadata[0].name
      }
    }
  }
}

# System service
resource "kubernetes_service" "system" {
  metadata {
    name = "system"
  }

  spec {
    selector = {
      app = "system"
    }

    port {
      port        = 80
      target_port = 80
      name        = "http"
    }

    port {
      port        = 8080
      target_port = 8080
      name        = "grpc-web"
    }

    type = "NodePort"
  }
}

# System ingress
resource "kubernetes_ingress_v1" "system" {
  metadata {
    name = "system"
    annotations = {
      "kubernetes.io/ingress.class"                       = "alb"
      "alb.ingress.kubernetes.io/ssl-redirect"            = "443"
      "alb.ingress.kubernetes.io/listen-ports"            = jsonencode([{ "HTTP" : 80 }, { "HTTPS" : 443 }])
      "alb.ingress.kubernetes.io/scheme"                  = "internet-facing"
      "alb.ingress.kubernetes.io/target-type"             = "ip"
      "alb.ingress.kubernetes.io/target-group-attributes" = "stickiness.enabled=true,stickiness.type=lb_cookie,stickiness.lb_cookie.duration_seconds=86400"
      "certificate-arn"                                   = "arn:aws:acm:eu-central-1:163349077661:certificate/8271c03a-0830-4c02-81af-b5add9429291"
    }
  }

  spec {
    tls {
      hosts       = ["system.justbench.com"]
      secret_name = "system-cert"
    }

    rule {
      host = "system.justbench.com"
      http {
        path {
          path      = "/"
          path_type = "Prefix"
          backend {
            service {
              name = kubernetes_service.system.metadata[0].name
              port {
                number = 8080
              }
            }
          }
        }
      }
    }
  }
}
