#
# Host
# 

locals {
  prefix = "bench-${var.env}-${var.cloud}-${var.region}"
  host_env_vars = {
    SERVICE_NAME = "host"
    ENVIRONMENT  = var.env
    CLOUD        = var.cloud
    REGION       = var.region

    SUPERVISOR_URL                = var.supervisor_url
    COMPUTER_RUNTIME_IMAGE        = "ghcr.io/symbolx/bench-runtime"
    KUBERNETES_NAMESPACE          = "default"
    KUBERNETES_COMPUTER_APP_LABEL = "bench-computer"
    KUBERNETES_IMAGE_PULL_SECRET  = kubernetes_secret.image_pull_secret.metadata[0].name

    GLOBAL_PG_URL = var.global_pg_url

    OTLP_ENDPOINT = "http://jaeger.monitoring.svc.cluster.local:4317"
    TRACING       = 1
    LOG_LEVEL     = "DEBUG"
    LOG_MODE      = "JSON"

    NEON_API_KEY        = var.neon_api_key
    NEON_BASE_URL       = var.neon_base_url
    OPENAI_API_KEY      = var.openai_api_key
    ANTHROPIC_API_KEY   = var.anthropic_api_key
    OPENROUTER_API_KEY  = var.openrouter_api_key
    XAI_API_KEY         = var.xai_api_key
    EXA_API_KEY         = var.exa_api_key
    UNSPLASH_ACCESS_KEY = var.unsplash_access_key
    GHCR_TOKEN          = var.ghcr_token
    S3_REGION           = aws_s3_bucket.bench_files.region
    S3_ENDPOINT         = "https://s3.${aws_s3_bucket.bench_files.region}.amazonaws.com"
    S3_ACCESS_KEY       = aws_iam_access_key.host.id
    S3_SECRET_KEY       = aws_iam_access_key.host.secret
  }
}

# host s3 access
resource "aws_iam_user" "host" {
  name = "bench-${var.env}-host"
}
resource "aws_iam_policy" "host_s3" {
  name        = "bench-${var.env}-${var.region}-host-s3"
  description = "Allow access to S3 buckets for host"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:ListBucket",
          "s3:PutObject",
          "s3:PutObjectAcl",
          "s3:DeleteObject",
          "s3:AbortMultipartUpload",
          "s3:ListMultipartUploadParts",
        ]
        Resource = [
          "arn:aws:s3:::bench-${var.env}*",
          "arn:aws:s3:::bench-${var.env}*/*",
        ]
      }
    ]
  })
}
resource "aws_iam_user_policy_attachment" "host_s3" {
  user       = aws_iam_user.host.name
  policy_arn = aws_iam_policy.host_s3.arn
}
resource "aws_iam_access_key" "host" {
  user = aws_iam_user.host.name
}

# host role / service account (to manage resources in cluster)
resource "kubernetes_service_account" "host" {
  metadata {
    name      = "${local.prefix}-host"
    namespace = "default"
  }
}
resource "kubernetes_cluster_role" "host" {
  metadata {
    name = "${local.prefix}-host"
  }

  rule {
    api_groups = [""]
    resources  = ["secrets", "configmaps", "pods", "services", "namespaces", "persistentvolumes", "persistentvolumeclaims"]
    verbs      = ["get", "list", "watch", "create", "update", "patch", "delete"]
  }
  rule {
    api_groups = ["apps"]
    resources  = ["deployments", "replicasets", "statefulsets"]
    verbs      = ["get", "list", "watch", "create", "update", "patch", "delete"]
  }
  rule {
    api_groups = ["batch"]
    resources  = ["jobs", "cronjobs"]
    verbs      = ["get", "list", "watch", "create", "update", "patch", "delete"]
  }
}
resource "kubernetes_cluster_role_binding" "host" {
  metadata {
    name = "${local.prefix}-host"
  }

  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "ClusterRole"
    name      = kubernetes_cluster_role.host.metadata[0].name
  }

  subject {
    kind      = "ServiceAccount"
    name      = kubernetes_service_account.host.metadata[0].name
    namespace = "default"
  }
}

# host deployment
resource "kubernetes_deployment" "host" {
  metadata {
    name      = "${local.prefix}-host"
    namespace = "default"
    labels = {
      app = "${local.prefix}-host"
    }
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "${local.prefix}-host"
      }
    }

    template {
      metadata {
        labels = {
          app = "${local.prefix}-host"
        }
        annotations = {
          "prometheus.io/scrape" = "true"
        }
      }

      spec {
        service_account_name = kubernetes_service_account.host.metadata[0].name
        container {
          name  = "host"
          image = "ghcr.io/symbolx/bench-system:${var.bench_version}"

          port {
            container_port = 60061
            name           = "grpc"
          }

          dynamic "env" {
            for_each = local.host_env_vars
            content {
              name  = env.key
              value = env.value
            }
          }

          command = ["python", "bench.py", "serve", "host", "0.0.0.0", "60061"]

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

# host service
resource "kubernetes_service" "host" {
  metadata {
    name = "${local.prefix}-host"
  }

  spec {
    selector = {
      app = "${local.prefix}-host"
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
# NOTE: right now this looks very similar to the host, but we'll probably need to shard this?
#

# Envoy ConfigMap
resource "kubernetes_config_map" "host_envoy_config" {
  metadata {
    name = "${local.prefix}-host-envoy-config"
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
                          cluster: host_service
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
            socket_address: { address: 0.0.0.0, port_value: ${var.host_grpc_port} }
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
                        cluster: host_service
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
          - name: host_service
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
                            address: ${local.prefix}-host
                            port_value: 60061
    EOT
  }
}

# Envoy Deployment
resource "kubernetes_deployment" "host_envoy_proxy" {
  metadata {
    name = "${local.prefix}-host-envoy-proxy"
    labels = {
      app = "${local.prefix}-host-envoy-proxy"
    }
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "${local.prefix}-host-envoy-proxy"
      }
    }

    template {
      metadata {
        labels = {
          app = "${local.prefix}-host-envoy-proxy"
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
            container_port = var.host_grpc_port
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
            name = kubernetes_config_map.host_envoy_config.metadata[0].name
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
resource "kubernetes_service" "host_envoy_proxy" {
  metadata {
    name = "${local.prefix}-host-envoy-proxy"
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
      app = "${local.prefix}-host-envoy-proxy"
    }

    port {
      name        = "grpc-web"
      port        = 443
      target_port = 8080
    }

    port {
      name        = "grpc"
      port        = var.host_grpc_port
      target_port = var.host_grpc_port
    }

    type = "LoadBalancer"
  }
}

data "kubernetes_service" "host_envoy_proxy" {
  metadata {
    name = kubernetes_service.host_envoy_proxy.metadata[0].name
  }

  depends_on = [kubernetes_service.host_envoy_proxy]
}

output "host_hostname" {
  value       = data.kubernetes_service.host_envoy_proxy.status.0.load_balancer.0.ingress.0.hostname
  description = "The public hostname of the load balancer"
}

# point 'host' for this region to the host service
resource "cloudflare_record" "host_region" {
  zone_id = var.web_zone_id
  name    = "${var.cloud}-${var.region}.host"
  type    = "CNAME"
  content = data.kubernetes_service.host_envoy_proxy.status.0.load_balancer.0.ingress.0.hostname
  ttl     = 300
  proxied = false
}
