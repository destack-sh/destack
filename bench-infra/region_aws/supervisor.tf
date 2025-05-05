# 
# Supervisor
# 

locals {
  supervisor_env_vars = {
    SERVICE_NAME  = "bench-supervisor"
    ENVIRONMENT   = var.env
    CLOUD         = var.cloud
    REGION        = var.region
    VERSION       = var.bench_version
    OTLP_ENDPOINT = "http://otel-collector.monitoring.svc.cluster.local:4317"
    TRACING       = 1
    LOG_LEVEL     = "DEBUG"
    LOG_MODE      = "JSON"
    USE_WAITLIST  = 1

    SUPERVISOR_URL = var.supervisor_url
    HOST_MAP = join(",", flatten([
      for k, v in var.host_map : [
        format("%s=%s", k, v)
      ]
    ]))

    COMPUTER_RUNTIME_IMAGE         = "ghcr.io/symbolx/bench-computer-runtime"
    COMPUTER_UBUNTU_DESKTOP_IMAGE  = "ghcr.io/symbolx/bench-computer-ubuntu-desktop"
    COMPUTER_UBUNTU_TERMINAL_IMAGE = "ghcr.io/symbolx/bench-computer-ubuntu-terminal"
    COMPUTER_GRPC_PORT             = 5432
    COMPUTER_VNC_PORT              = 6080
  }
  supervisor_secret_env_vars = {
    "${kubernetes_secret.db_secret.metadata[0].name}" = [
      "GLOBAL_PG_URL",
      "REGIONAL_PG_MAP"
    ]
    "${kubernetes_secret.external_secret.metadata[0].name}" = [
      "NEON_API_KEY",
      "NEON_BASE_URL",
      "POSTHOG_TOKEN",
      "POSTHOG_HOST",
    ]
    "${kubernetes_secret.supervisor_s3_secret.metadata[0].name}" = [
      "S3_REGION",
      "S3_ENDPOINT",
      "S3_ACCESS_KEY",
      "S3_SECRET_KEY"
    ]
  }
}

# supervisor s3 access
resource "kubernetes_secret" "supervisor_s3_secret" {
  metadata {
    name = "${local.prefix}-supervisor-s3-credentials"
  }

  data = {
    S3_REGION     = aws_s3_bucket.bench_files.region
    S3_ENDPOINT   = "https://s3.${aws_s3_bucket.bench_files.region}.amazonaws.com"
    S3_ACCESS_KEY = aws_iam_access_key.supervisor.id
    S3_SECRET_KEY = aws_iam_access_key.supervisor.secret
  }
}

# s3 access (to everything)
resource "aws_iam_user" "supervisor" {
  name = "bench-${var.env}-${var.region}-supervisor"
}
resource "aws_iam_policy" "supervisor_s3" {
  name        = "bench-${var.env}-${var.region}-supervisor-s3"
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
      app     = "bench-supervisor"
      env     = var.env
      cloud   = var.cloud
      region  = var.region
      version = var.bench_version
    }
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app     = "bench-supervisor"
        env     = var.env
        cloud   = var.cloud
        region  = var.region
        version = var.bench_version
      }
    }

    template {
      metadata {
        labels = {
          app     = "bench-supervisor"
          env     = var.env
          cloud   = var.cloud
          region  = var.region
          version = var.bench_version
        }
        annotations = {
          "prometheus.io/scrape" = "true"
        }
      }

      spec {
        # init container
        init_container {
          name    = "supervisor-init"
          image   = "ghcr.io/symbolx/bench-system:${var.bench_version}"
          command = ["/bin/sh", "-c"]
          # NOTE: migrating & bootstrapping in supervisor is scary if :MultiRegion
          args = [
            <<-EOT
              set -e
              python bench.py migrate apply --area global
              python bench.py system bootstrap --region ${var.region} --upsert
              python bench.py migrate apply --area regional --region ${var.region}
            EOT
          ]

          dynamic "env" {
            for_each = local.supervisor_env_vars
            content {
              name  = env.key
              value = env.value
            }
          }
          dynamic "env" {
            for_each = flatten([
              for secret_name, env_vars in local.supervisor_secret_env_vars : [
                for env_var in env_vars : {
                  name   = env_var
                  secret = secret_name
                }
              ]
            ])
            content {
              name = env.value.name
              value_from {
                secret_key_ref {
                  name = env.value.secret
                  key  = env.value.name
                }
              }
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
          dynamic "env" {
            for_each = flatten([
              for secret_name, env_vars in local.supervisor_secret_env_vars : [
                for env_var in env_vars : {
                  name   = env_var
                  secret = secret_name
                }
              ]
            ])
            content {
              name = env.value.name
              value_from {
                secret_key_ref {
                  name = env.value.secret
                  key  = env.value.name
                }
              }
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
            socket_address: { address: 0.0.0.0, port_value: 60061 }
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
            container_port = 60061
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
            secret_name = kubernetes_secret.web_certificate_secret.metadata[0].name
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
      port        = 60061
      target_port = 60061
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
  description = "The public hostname of the Supervisor service (ingress)"
}

# point 'supervisor' for this region to the supervisor ingress
resource "cloudflare_record" "supervisor" {
  zone_id         = var.web_zone_id
  name            = "${var.cloud}-${var.region}.supervisor"
  type            = "CNAME"
  content         = data.kubernetes_service.supervisor_envoy_proxy.status.0.load_balancer.0.ingress.0.hostname
  ttl             = 300
  proxied         = false
  allow_overwrite = true
}
