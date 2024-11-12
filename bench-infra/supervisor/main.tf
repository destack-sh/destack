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
  }
}

# 
# Supervisor
# 

locals {
  prefix = "bench-${var.env}-${var.cloud}-${var.region}"
  supervisor_env_vars = {
    SERVICE_NAME = "supervisor"
    ENVIRONMENT  = var.env
    CLOUD        = var.cloud
    REGION       = var.region

    SUPERVISOR_URL = "https://supervisor.justbench.com:${var.supervisor_grpc_port}"
    HOST_MAP = join(",", flatten([
      for k, v in var.host_map : [
        format("%s=%s", k, v)
      ]
    ]))
    MACHINE_RUNTIME_IMAGE = "ghcr.io/symbolx/bench-runtime"

    GLOBAL_PG_HOST       = var.global_pg_host
    GLOBAL_PG_NAME       = var.global_pg_name
    GLOBAL_PG_USERNAME   = var.global_pg_username
    GLOBAL_PG_PASSWORD   = var.global_pg_password
    GLOBAL_PG_CRYPTO_KEY = var.global_pg_crypto_key

    LOCAL_CACHE_HOST     = var.local_cache_host
    LOCAL_CACHE_USERNAME = var.local_cache_username
    LOCAL_CACHE_PASSWORD = var.local_cache_password

    OTLP_ENDPOINT = "http://jaeger.monitoring.svc.cluster.local:4317"
    TRACING       = 1
    LOG_LEVEL     = "DEBUG"
    LOG_MODE      = "JSON"
    USE_WAITLIST  = 1

    SENTRY_DSN    = var.sentry_dsn
    NEON_API_KEY  = var.neon_api_key
    NEON_BASE_URL = var.neon_base_url
    S3_ACCESS_KEY = aws_iam_access_key.supervisor.id
    S3_SECRET_KEY = aws_iam_access_key.supervisor.secret
  }
}

# s3 access (to everything)
resource "aws_iam_user" "supervisor" {
  name = "bench-${var.env}-supervisor"
}
resource "aws_iam_policy" "supervisor_s3" {
  name        = "bench-${var.env}-supervisor-s3"
  description = "Allow access to S3 buckets for supervisor"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:ListBucket",
          "s3:PutObject",
          "s3:DeleteObject",
        ]
        Resource = [
          "arn:aws:s3:::bench-${var.env}*",
          "arn:aws:s3:::bench-${var.env}*/*"
        ]
      }
    ]
  })
}
resource "aws_iam_user_policy_attachment" "supervisor_s3" {
  user       = aws_iam_user.supervisor.name
  policy_arn = aws_iam_policy.supervisor_s3.arn
}
resource "aws_iam_access_key" "supervisor" {
  user = aws_iam_user.supervisor.name
}

# Supervisor deployment
resource "kubernetes_deployment" "supervisor" {
  metadata {
    name      = "${local.prefix}-supervisor"
    namespace = "default"
    labels = {
      app = "${local.prefix}-supervisor"
    }
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "${local.prefix}-supervisor"
      }
    }

    template {
      metadata {
        labels = {
          app = "${local.prefix}-supervisor"
        }
        annotations = {
          "prometheus.io/scrape" = "true"
        }
      }

      spec {
        # init container
        init_container {
          name    = "supervisor-migrate"
          image   = "ghcr.io/symbolx/bench-system:${var.bench_version}"
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
          image = "ghcr.io/symbolx/bench-system:${var.bench_version}"

          port {
            container_port = 60061
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
    name = "${local.prefix}-supervisor"
  }

  spec {
    selector = {
      app = "${local.prefix}-supervisor"
    }

    port {
      port        = 60061
      target_port = 60061
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
    name = "${local.prefix}-supervisor-envoy-config"
  }

  data = {
    "envoy.yaml" = <<-EOT
      static_resources:
        listeners:
        - name: grpc_web_listener
          address:
            socket_address: { address: 0.0.0.0, port_value: 8080 }
          filter_chains:
            - filters:
              - name: envoy.filters.network.http_connection_manager
                typed_config:
                  "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
                  codec_type: auto
                  stat_prefix: ingress_http
                  stream_idle_timeout: 0s
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
                      typed_config:
                        "@type": type.googleapis.com/envoy.extensions.filters.http.grpc_web.v3.GrpcWeb
                    - name: envoy.filters.http.cors
                      typed_config:
                        "@type": type.googleapis.com/envoy.extensions.filters.http.cors.v3.Cors
                    - name: envoy.filters.http.router
                      typed_config:
                        "@type": type.googleapis.com/envoy.extensions.filters.http.router.v3.Router
                  access_log:
                    - name: envoy.access_loggers.stdout
                      typed_config:
                        "@type": type.googleapis.com/envoy.extensions.access_loggers.stream.v3.StdoutAccessLog
                        log_format:
                          text_format: "[%START_TIME%] \"%REQ(:METHOD)% %REQ(X-ENVOY-ORIGINAL-PATH?:PATH)% %PROTOCOL%\" %RESPONSE_CODE% %RESPONSE_FLAGS% %BYTES_RECEIVED% %BYTES_SENT% %DURATION% %RESP(X-ENVOY-UPSTREAM-SERVICE-TIME)% \"%REQ(X-FORWARDED-FOR)%\" \"%REQ(USER-AGENT)%\" \"%REQ(X-REQUEST-ID)%\" \"%REQ(:AUTHORITY)%\" \"%UPSTREAM_HOST%\"\n"
              # tls to enable h2
              transport_socket:
                name: envoy.tls_context.http3
                typed_config:
                  "@type": type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.DownstreamTlsContext
                  common_tls_context:
                    alpn_protocols: ["h2"]
                    tls_certificates:
                      - certificate_chain:
                          filename: /etc/envoy/tls/tls.crt
                        private_key:
                          filename: /etc/envoy/tls/tls.key
        - name: grpc_listener
          address:
            socket_address: { address: 0.0.0.0, port_value: ${var.supervisor_grpc_port} }
          filter_chains:
          - filters:
            - name: envoy.filters.network.http_connection_manager
              typed_config:
                "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
                codec_type: auto
                stat_prefix: grpc_json
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
                http_filters:
                - name: envoy.filters.http.router
                  typed_config:
                    "@type": type.googleapis.com/envoy.extensions.filters.http.router.v3.Router
                access_log:
                - name: envoy.access_loggers.stdout
                  typed_config:
                    "@type": type.googleapis.com/envoy.extensions.access_loggers.stream.v3.StdoutAccessLog
                    log_format:
                      text_format: "[%START_TIME%] \"%REQ(:METHOD)% %REQ(X-ENVOY-ORIGIN)% %REQ(:AUTHORITY)% %UPSTREAM_HOST%\" %RESP(STATUS)% %BYTES_RECEIVED% %BYTES_SENT% %DURATION% %RESP(X-ENVOY-UPSTREAM-SERVICE-TIME)% \"%REQ(X-FORWARDED-FOR)%\" \"%REQ(USER-AGENT)%\" \"%REQ(X-REQUEST-ID)%\" \"%REQ(:AUTHORITY)%\" \"%UPSTREAM_HOST%\"\n"
            transport_socket:
              name: envoy.transport_sockets.tls
              typed_config:
                "@type": type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.DownstreamTlsContext
                common_tls_context:
                  alpn_protocols: ["h2"]
                  tls_certificates:
                    - certificate_chain:
                        filename: /etc/envoy/tls/tls.crt
                      private_key:
                        filename: /etc/envoy/tls/tls.key
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
                            address: ${local.prefix}-supervisor
                            port_value: 60061
    EOT
  }
}

# Envoy Deployment
resource "kubernetes_deployment" "supervisor_envoy_proxy" {
  metadata {
    name = "${local.prefix}-supervisor-envoy-proxy"
    labels = {
      app = "${local.prefix}-supervisor-envoy-proxy"
    }
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "${local.prefix}-supervisor-envoy-proxy"
      }
    }

    template {
      metadata {
        labels = {
          app = "${local.prefix}-supervisor-envoy-proxy"
        }
      }

      spec {
        container {
          image = "envoyproxy/envoy:v1.31.0"
          name  = "envoy"

          port {
            container_port = 8080
          }
          port {
            container_port = var.supervisor_grpc_port
          }

          volume_mount {
            name       = "envoy-config"
            mount_path = "/etc/envoy"
            read_only  = true
          }

          volume_mount {
            name       = "envoy-cert"
            mount_path = "/etc/envoy/tls"
            read_only  = true
          }
        }

        volume {
          name = "envoy-config"
          config_map {
            name = kubernetes_config_map.supervisor_envoy_config.metadata[0].name
          }
        }

        volume {
          name = "envoy-cert"
          secret {
            secret_name = var.web_certificate_secret_name
            items {
              key  = "tls.crt"
              path = "tls.crt"
            }
            items {
              key  = "tls.key"
              path = "tls.key"
            }
          }
        }
      }
    }
  }
}

# Envoy Service
resource "kubernetes_service" "supervisor_envoy_proxy" {
  metadata {
    name = "${local.prefix}-supervisor-envoy-proxy"
    annotations = {
      "service.beta.kubernetes.io/aws-load-balancer-type"                            = "nlb"
      "service.beta.kubernetes.io/aws-load-balancer-nlb-target-type"                 = "ip"
      "service.beta.kubernetes.io/aws-load-balancer-scheme"                          = "internet-facing"
      "service.beta.kubernetes.io/aws-load-balancer-backend-protocol"                = "ssl"
      "service.beta.kubernetes.io/aws-load-balancer-name"                            = "${local.prefix}-nlb"
      "service.beta.kubernetes.io/aws-load-balancer-healthcheck-protocol"            = "TCP"
      "service.beta.kubernetes.io/aws-load-balancer-healthcheck-healthy-threshold"   = "2"
      "service.beta.kubernetes.io/aws-load-balancer-healthcheck-unhealthy-threshold" = "2"
      "service.beta.kubernetes.io/aws-load-balancer-healthcheck-interval"            = "10"
      "service.beta.kubernetes.io/aws-load-balancer-healthcheck-timeout"             = "5"
    }
  }

  spec {
    selector = {
      app = "${local.prefix}-supervisor-envoy-proxy"
    }

    port {
      name        = "grpc-web"
      port        = 443
      target_port = 8080
    }

    port {
      name        = "grpc"
      port        = var.supervisor_grpc_port
      target_port = var.supervisor_grpc_port
    }

    type = "LoadBalancer"
  }
}

data "kubernetes_service" "supervisor_envoy_proxy" {
  metadata {
    name = kubernetes_service.supervisor_envoy_proxy.metadata[0].name
  }

  depends_on = [kubernetes_service.supervisor_envoy_proxy]
}

output "supervisor_hostname" {
  value       = data.kubernetes_service.supervisor_envoy_proxy.status.0.load_balancer.0.ingress.0.hostname
  description = "The public hostname of the load balancer"
}
