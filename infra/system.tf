# EKS (system) nodes
resource "aws_eks_node_group" "eks_system_nodes" {
  cluster_name    = aws_eks_cluster.eks_cluster.name
  node_group_name = "${var.env}-eks-system-nodes"
  node_role_arn   = aws_iam_role.eks_node_role.arn
  subnet_ids      = aws_subnet.private[*].id

  scaling_config {
    desired_size = var.system_desired_cluster_size
    max_size     = var.system_max_cluster_size
    min_size     = var.system_min_cluster_size
  }

  instance_types = [var.system_node_instance_type]
}


#
# AWS RDS Aurora 
# 

resource "aws_security_group" "rds_security_group" {
  name        = "${var.env}-rds-security-group"
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

resource "aws_db_subnet_group" "rds_subnet_group" {
  name       = "${var.env}-rds-subnet-group"
  subnet_ids = aws_subnet.private[*].id

  tags = {
    Name = "${var.env}-rds-subnet-group"
  }
}

resource "aws_rds_cluster" "global_pg" {
  cluster_identifier      = "${var.env}-global-db"
  engine                  = "aurora-postgresql"
  engine_mode             = "provisioned"
  engine_version          = "16.2"
  database_name           = var.global_pg_name
  master_username         = var.global_pg_username
  master_password         = var.global_pg_password
  backup_retention_period = 7
  preferred_backup_window = "06:00-08:00"
  storage_encrypted       = true

  vpc_security_group_ids = [aws_security_group.rds_security_group.id]
  db_subnet_group_name   = aws_db_subnet_group.rds_subnet_group.name
}

resource "aws_rds_cluster_instance" "global_pg_instance" {
  count                      = 1
  identifier                 = "${var.env}-global-db-${count.index}"
  cluster_identifier         = aws_rds_cluster.global_pg.id
  instance_class             = "db.t3.medium"
  engine                     = aws_rds_cluster.global_pg.engine
  engine_version             = aws_rds_cluster.global_pg.engine_version
  auto_minor_version_upgrade = true
}


# 
# System (Supervisor/Host)
# NOTE :Infra! :Robustness: deploy Supervisor and Host separately
# 

# # envoy
# resource "kubernetes_config_map" "envoy_config" {
#   metadata {
#     name      = "envoy-config"
#     namespace = "default"
#   }

#   data = {
#     "envoy.yaml" = file("envoy.yaml")
#   }
# }

# # System deployment
# resource "kubernetes_deployment" "system" {
#   metadata {
#     name      = "system"
#     namespace = "default"
#     labels = {
#       app = "system"
#     }
#   }

#   spec {
#     replicas = 1

#     selector {
#       match_labels = {
#         app = "system"
#       }
#     }

#     template {
#       metadata {
#         labels = {
#           app = "system"
#         }
#         annotations = {
#           "prometheus.io/scrape" = "true"
#         }
#       }

#       spec {
#         init_container {
#           name    = "system-migrate"
#           image   = "ghcr.io/symbolx/bench-system:${var.version}"
#           command = ["/bin/sh", "-c"]
#           args    = ["python bench.py migrate apply"]

#           env {
#             name  = "ENVIRONMENT"
#             value = var.env
#           }
#           # Add other environment variables as needed
#         }

#         container {
#           name  = "envoy"
#           image = "envoyproxy/envoy:v1.28-latest"
#           port {
#             container_port = 8080
#             name           = "grpc-web"
#           }
#           volume_mount {
#             name       = "envoy-config"
#             mount_path = "/etc/envoy"
#             read_only  = true
#           }
#         }

#         container {
#           name  = "system"
#           image = "ghcr.io/symbolx/bench-system:${var.version}"

#           port {
#             container_port = 80
#             name           = "http"
#           }
#           port {
#             container_port = 50051
#             name           = "grpc"
#           }

#           env {
#             name  = "ENVIRONMENT"
#             value = var.env
#           }
#           env {
#             name  = "REGION"
#             value = var.aws_region
#           }

#           env {
#             name  = "GLOBAL_PG_HOST"
#             value = aws_rds_cluster.global_pg.endpoint
#           }
#           env {
#             name  = "GLOBAL_PG_PASSWORD"
#             value = var.global_pg_password
#           }
#           env {
#             name  = "GLOBAL_PG_CRYPTO_KEY"
#             value = var.global_pg_crypto_key
#           }

#           env {
#             name  = "CORS_ALLOWED_HOSTS"
#             value = var.cors_allowed_hosts
#           }
#           env {
#             name  = "CORS_ALLOWED_ORIGINS"
#             value = var.cors_allowed_origins
#           }

#           env {
#             name  = "SENTRY_DSN"
#             value = var.sentry_dsn
#           }
#           env {
#             name  = "OPENAI_API_KEY"
#             value = var.openai_api_key
#           }
#           env {
#             name  = "ANTHROPIC_API_KEY"
#             value = var.anthropic_api_key
#           }

#           command = ["python", "bench.py", "serve", "system"]

#           resources {
#             requests = {
#               cpu    = "2000m"
#               memory = "2000Mi"
#             }
#           }
#         }

#         volume {
#           name = "envoy-config"
#           config_map {
#             name = kubernetes_config_map.envoy_config.metadata[0].name
#           }
#         }

#         image_pull_secrets {
#           name = kubernetes_secret.image_pull_secret.metadata[0].name
#         }

#         service_account_name = kubernetes_service_account.system_service_account.metadata[0].name
#       }
#     }
#   }
# }

# # System service
# resource "kubernetes_service" "system" {
#   metadata {
#     name = "system"
#   }

#   spec {
#     selector = {
#       app = "system"
#     }

#     port {
#       port        = 80
#       target_port = 80
#       name        = "http"
#     }

#     port {
#       port        = 8080
#       target_port = 8080
#       name        = "grpc-web"
#     }

#     type = "NodePort"
#   }
# }

# # System ingress
# resource "kubernetes_ingress_v1" "system" {
#   metadata {
#     name = "system"
#     annotations = {
#       "kubernetes.io/ingress.class"                       = "alb"
#       "alb.ingress.kubernetes.io/ssl-redirect"            = "443"
#       "alb.ingress.kubernetes.io/listen-ports"            = jsonencode([{ "HTTP" : 80 }, { "HTTPS" : 443 }])
#       "alb.ingress.kubernetes.io/scheme"                  = "internet-facing"
#       "alb.ingress.kubernetes.io/target-type"             = "ip"
#       "alb.ingress.kubernetes.io/target-group-attributes" = "stickiness.enabled=true,stickiness.type=lb_cookie,stickiness.lb_cookie.duration_seconds=86400"
#       "certificate-arn"                                   = "arn:aws:acm:eu-central-1:163349077661:certificate/8271c03a-0830-4c02-81af-b5add9429291"
#     }
#   }

#   spec {
#     tls {
#       hosts       = ["system.justbench.com"]
#       secret_name = "system-cert"
#     }

#     rule {
#       host = "system.justbench.com"
#       http {
#         path {
#           path      = "/"
#           path_type = "Prefix"
#           backend {
#             service {
#               name = kubernetes_service.system.metadata[0].name
#               port {
#                 number = 8080
#               }
#             }
#           }
#         }
#       }
#     }
#   }
# }
