
#
# S3 buckets
# 

resource "aws_s3_bucket" "bench_public" {
  bucket = "bench-${var.env}-${var.region}-public"
  tags = {
    Name = "bench-${var.env}-${var.region}-public"
  }
}


# 
# EKS (system) nodes
#

resource "aws_eks_node_group" "region_system_nodes" {
  cluster_name    = aws_eks_cluster.region_cluster.name
  node_group_name = "bench-${var.env}-${var.region}-region-system-nodes"
  node_role_arn   = aws_iam_role.region_node_role.arn
  subnet_ids      = aws_subnet.private[*].id

  scaling_config {
    desired_size = var.system_desired_cluster_size
    max_size     = var.system_max_cluster_size
    min_size     = var.system_min_cluster_size
  }

  instance_types = [var.system_node_instance_type]
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
#           image   = "ghcr.io/symbolx/bench-system:${var.bench_version}"
#           command = ["/bin/sh", "-c"]
#           args    = ["python bench.py migrate apply"]

#           env {
#             name  = "ENVIRONMENT"
#             value = var.env
#           }
#           env {
#             name  = "REGION"
#             value = var.region
#           }

#           env {
#             name  = "GLOBAL_PG_HOST"
#             value = var.global_pg_host
#           }
#           env {
#             name  = "GLOBAL_PG_USERNAME"
#             value = var.global_pg_username
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
#             name  = "SENTRY_DSN"
#             value = var.sentry_dsn
#           }
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
#           image = "ghcr.io/symbolx/bench-system:${var.bench_version}"

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
#             value = var.region
#           }

#           env {
#             name  = "GLOBAL_PG_HOST"
#             value = var.global_pg_host
#           }
#           env {
#             name  = "GLOBAL_PG_USERNAME"
#             value = var.global_pg_username
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
#             name  = "SENTRY_DSN"
#             value = var.sentry_dsn
#           }
#           env {
#             name  = "NEON_API_KEY"
#             value = var.neon_api_key
#           }
#           env {
#             name  = "NEON_BASE_URL"
#             value = var.neon_base_url
#           }
#           env {
#             name  = "OPENAI_API_KEY"
#             value = var.openai_api_key
#           }
#           env {
#             name  = "ANTHROPIC_API_KEY"
#             value = var.anthropic_api_key
#           }
#           env {
#             name  = "GHCR_TOKEN"
#             value = var.ghcr_token
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
