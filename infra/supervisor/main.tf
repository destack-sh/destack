
# 
# Supervisor
# 

locals {
  supervisor_env_vars = {
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

    SENTRY_DSN    = var.sentry_dsn
    NEON_API_KEY  = var.neon_api_key
    NEON_BASE_URL = var.neon_base_url
  }
}

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

          dynamic "env" {
            for_each = local.supervisor_env_vars
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
            for_each = local.supervisor_env_vars
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
          name = var.image_pull_secret_name
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

# 
# Envoy proxy
#


# Envoy ConfigMap
resource "kubernetes_config_map" "supervisor_envoy_config" {
  metadata {
    name = "bench-${var.env}-${var.cloud}-${var.region}-supervisor-envoy-config"
  }

  data = {
    "envoy.yaml" = <<-EOT
      static_resources:
        listeners:
        - name: listener_0
          address:
            socket_address: { address: 0.0.0.0, port_value: 8080 }
          filter_chains:
          - filters:
            - name: envoy.filters.network.http_connection_manager
              typed_config:
                "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
                codec_type: auto
                stat_prefix: ingress_http
                route_config:
                  name: local_route
                  virtual_hosts:
                  - name: local_service
                    domains: ["*"]
                    routes:
                    - match: { prefix: "/" }
                      route:
                        cluster: supervisor_service
                        timeout: 0s
                        max_stream_duration:
                          grpc_timeout_header_max: 0s
                    cors:
                      allow_origin_string_match:
                        - prefix: "*"
                      allow_methods: GET, PUT, DELETE, POST, OPTIONS
                      allow_headers: keep-alive,user-agent,cache-control,content-type,content-transfer-encoding,x-accept-content-transfer-encoding,x-accept-response-streaming,x-user-agent,grpc-web,x-grpc-web,grpc-timeout,grpc-timeout,x-bench-1,x-bench-2,x-bench-3,x-bench-4,x-bench-5,x-bench-6,x-bench-7,x-bench-8,x-bench-9
                      max_age: "1728000"
                      expose_headers: content-type,grpc-status,grpc-message,grpc-web,x-grpc-web,x-bench-1,x-bench-2,x-bench-3,x-bench-4,x-bench-5,x-bench-6,x-bench-7,x-bench-8,x-bench-9
                http_filters:
                  - name: envoy.filters.http.grpc_web
                  - name: envoy.filters.http.cors
                  - name: envoy.filters.http.router
                # tls to enable h2
                transport_socket:
                  name: envoy.tls_context.http3
                  typed_config:
                    "@type": type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.DownstreamTlsContext
                    common_tls_context:
                      alpn_protocols: ["h2"]
                      tls_certificates:
                        - certificate_chain:
                            filename: /etc/envoy/ssl/cert.pem
                          private_key:
                            filename: /etc/envoy/ssl/cert.key
        clusters:
        - name: supervisor_service
          connect_timeout: 0.25s
          type: logical_dns
          http2_protocol_options: {}
          lb_policy: round_robin
          load_assignment:
            cluster_name: cluster_0
            endpoints:
              - lb_endpoints:
                  - endpoint:
                      address:
                        socket_address:
                          address: bench-${var.env}-${var.cloud}-${var.region}-supervisor
                          port_value: 60051
    EOT
  }
}

# Envoy Deployment
resource "kubernetes_deployment" "supervisor_envoy_proxy" {
  metadata {
    name = "bench-${var.env}-${var.cloud}-${var.region}-supervisor-envoy-proxy"
    labels = {
      app = "bench-${var.env}-${var.cloud}-${var.region}-supervisor-envoy-proxy"
    }
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "bench-${var.env}-${var.cloud}-${var.region}-supervisor-envoy-proxy"
      }
    }

    template {
      metadata {
        labels = {
          app = "bench-${var.env}-${var.cloud}-${var.region}-supervisor-envoy-proxy"
        }
      }

      spec {
        container {
          image = "envoyproxy/envoy:v1.31.0"
          name  = "envoy"

          port {
            container_port = 443
          }

          volume_mount {
            name       = "bench-${var.env}-${var.cloud}-${var.region}-supervisor-envoy-config"
            mount_path = "/etc/envoy"
            read_only  = true
          }
        }

        volume {
          name = "envoy-config"
          config_map {
            name = kubernetes_config_map.supervisor_envoy_config.metadata[0].name
          }
        }
      }
    }
  }
}

# Envoy Service
resource "kubernetes_service" "envoy_proxy" {
  metadata {
    name = "bench-${var.env}-${var.cloud}-${var.region}-supervisor-envoy-proxy"
  }

  spec {
    selector = {
      app = "bench-${var.env}-${var.cloud}-${var.region}-supervisor-envoy-proxy"
    }

    port {
      port        = 443
      target_port = 443
    }

    type = "NodePort"
  }
}

#
# Supervisor ingress
# nocheckin
# 

# # ALB Security Group
# resource "aws_security_group" "alb_sg" {
#   name        = "alb-supervisor-sg"
#   description = "Security group for ALB"
#   vpc_id      = var.vpc_id

#   ingress {
#     from_port   = 443
#     to_port     = 443
#     protocol    = "tcp"
#     cidr_blocks = ["0.0.0.0/0"]
#   }

#   egress {
#     from_port   = 0
#     to_port     = 0
#     protocol    = "-1"
#     cidr_blocks = ["0.0.0.0/0"]
#   }
# }

# # ALB
# resource "aws_lb" "supervisor_alb" {
#   name               = "supervisor-alb"
#   internal           = false
#   load_balancer_type = "application"
#   security_groups    = [aws_security_group.alb_sg.id]
#   subnets            = var.public_subnet_ids

#   enable_deletion_protection = false
# }

# # ALB Listener
# resource "aws_lb_listener" "front_end" {
#   load_balancer_arn = aws_lb.supervisor_alb.arn
#   port              = "443"
#   protocol          = "HTTPS"
#   ssl_policy        = "ELBSecurityPolicy-2016-08"

#   default_action {
#     type             = "forward"
#     target_group_arn = aws_lb_target_group.supervisor_tg.arn
#   }
# }

# # ALB Target Group
# resource "aws_lb_target_group" "supervisor_tg" {
#   name        = "supervisor-tg"
#   port        = 8080
#   protocol    = "HTTP"
#   vpc_id      = var.vpc_id
#   target_type = "ip"

#   health_check {
#     path                = "/healthz" # Adjust this to a proper health check endpoint
#     healthy_threshold   = 2
#     unhealthy_threshold = 10
#   }
# }

# # Attach the Envoy service to the target group
# resource "aws_lb_target_group_attachment" "supervisor_tg_attachment" {
#   target_group_arn = aws_lb_target_group.supervisor_tg.arn
#   target_id        = kubernetes_service.envoy_proxy.status.0.load_balancer.0.ingress.0.hostname
#   port             = 8080
# }
