
# 
# Supervisor
# 

locals {
  env_vars = {
    SERVICE_NAME = "supervisor"
    ENVIRONMENT  = var.env
    CLOUD        = var.cloud
    REGION       = var.region
    # encode host map as k=v,k=v,...
    HOST_MAP = join(",", flatten([
      for k, v in var.host_map : [
        format("%s=%s", k, v)
      ]
    ]))

    GLOBAL_PG_HOST       = var.global_pg_host
    GLOBAL_PG_NAME       = var.global_pg_name
    GLOBAL_PG_USERNAME   = var.global_pg_username
    GLOBAL_PG_PASSWORD   = var.global_pg_password
    GLOBAL_PG_CRYPTO_KEY = var.global_pg_crypto_key

    TRACING   = 0
    LOG_LEVEL = "DEBUG"
    LOG_MODE  = "JSON"

    SENTRY_DSN        = var.sentry_dsn
    NEON_API_KEY      = var.neon_api_key
    NEON_BASE_URL     = var.neon_base_url
    OPENAI_API_KEY    = var.openai_api_key
    ANTHROPIC_API_KEY = var.anthropic_api_key
    GHCR_TOKEN        = var.ghcr_token
  }
}

# Supervisor deployment
resource "kubernetes_deployment" "supervisor" {
  count = var.is_primary ? 1 : 0

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

          dynamic "env" {
            for_each = local.env_vars
            content {
              name  = env.key
              value = env.value
            }
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

          dynamic "env" {
            for_each = local.env_vars
            content {
              name  = env.key
              value = env.value
            }
          }

          command = ["python", "bench.py", "serve", "supervisor", "0.0.0.0", "60061"]

          resources {
            requests = {
              cpu    = "500m"
              memory = "500Mi"
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
  count = var.is_primary ? 1 : 0

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
#       hosts       = ["supervisor.${local.main_website}"]
#       secret_name = "supervisor-cert"
#     }

#     rule {
#       host = "supervisor.${local.main_website}"
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
