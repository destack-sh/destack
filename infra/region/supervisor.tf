
# 
# Supervisor
# 

# Supervisor deployment
resource "kubernetes_deployment" "supervisor" {
  metadata {
    name      = "bench-${var.env}-${var.cloud}-${var.region}-supervisor"
    namespace = "default"
    labels = {
      app = "bench-${var.env}-${var.cloud}-${var.region}-supervisor"
    }
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "bench-${var.env}-${var.cloud}-${var.region}-supervisor"
      }
    }

    template {
      metadata {
        labels = {
          app = "bench-${var.env}-${var.cloud}-${var.region}-supervisor"
        }
        annotations = {
          "prometheus.io/scrape" = "true"
        }
      }

      spec {
        # init container
        init_container {
          name    = "supervisor-migrate"
          image   = "ghcr.io/symbolx/bench-system:${var.git_commit}"
          command = ["/bin/sh", "-c"]
          args    = ["python bench.py migrate apply"]

          env {
            name  = "SERVICE_NAME"
            value = "supervisor"
          }
          env {
            name  = "ENVIRONMENT"
            value = var.env
          }
          env {
            name  = "REGION"
            value = var.region
          }

          env {
            name  = "GLOBAL_PG_HOST"
            value = var.global_pg_host
          }
          env {
            name  = "GLOBAL_PG_USERNAME"
            value = var.global_pg_username
          }
          env {
            name  = "GLOBAL_PG_PASSWORD"
            value = var.global_pg_password
          }
          env {
            name  = "GLOBAL_PG_CRYPTO_KEY"
            value = var.global_pg_crypto_key
          }

          # TODO :Infra: enable tracing
          env {
            name  = "TRACING"
            value = 0
          }
          env {
            name  = "LOG_LEVEL"
            value = "DEBUG"
          }
          env {
            name  = "LOG_MODE"
            value = "JSON"
          }

          env {
            name  = "SENTRY_DSN"
            value = var.sentry_dsn
          }
        }

        # main container
        container {
          name  = "supervisor"
          image = "ghcr.io/symbolx/bench-system:${var.git_commit}"

          port {
            container_port = 80
            name           = "http"
          }
          port {
            container_port = 60051
            name           = "grpc"
          }

          env {
            name  = "SERVICE_NAME"
            value = "supervisor"
          }
          env {
            name  = "ENVIRONMENT"
            value = var.env
          }
          env {
            name  = "REGION"
            value = var.region
          }

          env {
            name  = "GLOBAL_PG_HOST"
            value = var.global_pg_host
          }
          env {
            name  = "GLOBAL_PG_USERNAME"
            value = var.global_pg_username
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
            name  = "TRACING"
            value = 0
          }
          env {
            name  = "LOG_LEVEL"
            value = "DEBUG"
          }
          env {
            name  = "LOG_MODE"
            value = "JSON"
          }

          env {
            name  = "SENTRY_DSN"
            value = var.sentry_dsn
          }
          env {
            name  = "NEON_API_KEY"
            value = var.neon_api_key
          }
          env {
            name  = "NEON_BASE_URL"
            value = var.neon_base_url
          }
          env {
            name  = "OPENAI_API_KEY"
            value = var.openai_api_key
          }
          env {
            name  = "ANTHROPIC_API_KEY"
            value = var.anthropic_api_key
          }
          env {
            name  = "GHCR_TOKEN"
            value = var.ghcr_token
          }

          command = ["python", "bench.py", "serve", "supervisor"]

          resources {
            requests = {
              cpu    = "500m"
              memory = "1000Mi"
            }
          }
        }

        image_pull_secrets {
          name = kubernetes_secret.image_pull_secret.metadata[0].name
        }
      }
    }
  }
}

# Supervisor service
resource "kubernetes_service" "supervisor" {
  metadata {
    name = "bench-${var.env}-${var.cloud}-${var.region}-supervisor"
  }

  spec {
    selector = {
      app = "bench-${var.env}-${var.cloud}-${var.region}-supervisor"
    }

    port {
      port        = 60051
      target_port = 60051
      name        = "grpc"
    }

    type = "NodePort"
  }
}

# # Supervisor ingress
# resource "kubernetes_ingress_v1" "supervisor" {
#   metadata {
#     name = "supervisor"
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
#       hosts       = ["supervisor.justbench.com"]
#       secret_name = "supervisor-cert"
#     }

#     rule {
#       host = "supervisor.justbench.com"
#       http {
#         path {
#           path      = "/"
#           path_type = "Prefix"
#           backend {
#             service {
#               name = kubernetes_service.supervisor.metadata[0].name
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
