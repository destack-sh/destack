resource "kubernetes_namespace" "monitoring" {
  metadata {
    name = "monitoring"
  }
}

#
# Metrics server
#

resource "kubernetes_service_account" "metrics_server" {
  metadata {
    name      = "metrics-server"
    namespace = "kube-system"
  }
}

resource "kubernetes_cluster_role" "metrics_server" {
  metadata {
    name = "system:metrics-server"
  }

  rule {
    api_groups = [""]
    resources  = ["*"]
    verbs      = ["get", "list", "watch"]
  }
}

resource "kubernetes_cluster_role_binding" "metrics_server" {
  metadata {
    name = "system:metrics-server"
  }
  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "ClusterRole"
    name      = "system:metrics-server"
  }
  subject {
    kind      = "ServiceAccount"
    name      = "metrics-server"
    namespace = "kube-system"
  }
}

resource "kubernetes_deployment" "metrics_server" {
  metadata {
    name      = "metrics-server"
    namespace = "kube-system"
  }
  spec {
    replicas = 1
    selector {
      match_labels = {
        k8s-app = "metrics-server"
      }
    }
    template {
      metadata {
        labels = {
          k8s-app = "metrics-server"
        }
      }
      spec {
        service_account_name = kubernetes_service_account.metrics_server.metadata[0].name
        container {
          name  = "metrics-server"
          image = "bitnami/metrics-server:0.7.1"

          args = [
            "--cert-dir=/tmp",
            "--secure-port=4443",
            "--kubelet-preferred-address-types=InternalIP,Hostname,InternalDNS,ExternalDNS,ExternalIP",
            "--kubelet-use-node-status-port",
            "--metric-resolution=15s",
          ]

          port {
            container_port = 4443
            name           = "https"
          }
          liveness_probe {
            http_get {
              path   = "/livez"
              port   = "https"
              scheme = "HTTPS"
            }
            initial_delay_seconds = 20
            timeout_seconds       = 20
          }
          readiness_probe {
            http_get {
              path   = "/readyz"
              port   = "https"
              scheme = "HTTPS"
            }
            initial_delay_seconds = 20
            timeout_seconds       = 20
          }
        }
      }
    }
  }
}

#
# kube-state-metrics
#

resource "kubernetes_service_account" "kube_state_metrics" {
  metadata {
    name      = "kube-state-metrics"
    namespace = "kube-system"
  }
}

resource "kubernetes_cluster_role" "kube_state_metrics" {
  metadata {
    name = "kube-state-metrics"
  }
  rule {
    api_groups = ["*"]
    resources  = ["*"]
    verbs      = ["get", "list", "watch"]
  }
}

resource "kubernetes_cluster_role_binding" "kube_state_metrics" {
  metadata {
    name = "kube-state-metrics"
  }
  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "ClusterRole"
    name      = "kube-state-metrics"
  }
  subject {
    kind      = "ServiceAccount"
    name      = "kube-state-metrics"
    namespace = "kube-system"
  }
}

# kube-state-metrics deployment
resource "kubernetes_deployment" "kube_state_metrics" {
  metadata {
    name      = "kube-state-metrics"
    namespace = "kube-system"
  }
  spec {
    replicas = 1
    selector {
      match_labels = {
        app = "kube-state-metrics"
      }
    }
    template {
      metadata {
        labels = {
          app = "kube-state-metrics"
        }
      }
      spec {
        service_account_name = kubernetes_service_account.kube_state_metrics.metadata[0].name
        container {
          name  = "kube-state-metrics"
          image = "bitnami/kube-state-metrics:2.13.0"

          port {
            container_port = 8080
            name           = "http-metrics"
          }
          port {
            container_port = 8081
            name           = "telemetry"
          }
        }
      }
    }
  }
}

# kube-state-metrics service
resource "kubernetes_service" "kube_state_metrics" {
  metadata {
    name      = "kube-state-metrics"
    namespace = "kube-system"
    labels = {
      app = "kube-state-metrics"
    }
  }
  spec {
    selector = {
      app = "kube-state-metrics"
    }
    port {
      port        = 8080
      target_port = 8080
      protocol    = "TCP"
      name        = "http-metrics"
    }
    port {
      port        = 8081
      target_port = 8081
      protocol    = "TCP"
      name        = "telemetry"
    }
  }
}

#
# Grafana Cloud
#

# Grafana Cloud credentials secret
resource "kubernetes_secret" "grafana_cloud_secret" {
  metadata {
    name      = "grafana-cloud-credentials"
    namespace = kubernetes_namespace.monitoring.metadata[0].name
  }

  data = {
    "url"      = "https://logs-prod-039.grafana.net/loki/api/v1/push"
    "username" = "1203865"
    "password" = var.grafana_cloud_token
  }
}

#
# Promtail
#

# Promtail service account
resource "kubernetes_service_account" "promtail" {
  metadata {
    name      = "promtail"
    namespace = kubernetes_namespace.monitoring.metadata[0].name
  }
}

# Promtail cluster role
resource "kubernetes_cluster_role" "promtail" {
  metadata {
    name = "promtail"
  }

  rule {
    api_groups = [""]
    resources  = ["nodes", "nodes/proxy", "services", "endpoints", "pods"]
    verbs      = ["get", "watch", "list"]
  }
}

# Promtail cluster role binding
resource "kubernetes_cluster_role_binding" "promtail" {
  metadata {
    name = "promtail"
  }
  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "ClusterRole"
    name      = kubernetes_cluster_role.promtail.metadata[0].name
  }
  subject {
    kind      = "ServiceAccount"
    name      = kubernetes_service_account.promtail.metadata[0].name
    namespace = kubernetes_namespace.monitoring.metadata[0].name
  }
}

# Promtail config map
resource "kubernetes_config_map" "promtail_config" {
  metadata {
    name      = "promtail-config"
    namespace = kubernetes_namespace.monitoring.metadata[0].name
  }

  data = {
    "promtail.yaml" = <<-EOT
      server:
        http_listen_port: 9080
        grpc_listen_port: 0

      positions:
        filename: /run/promtail/positions.yaml

      clients:
        - url: ${kubernetes_secret.grafana_cloud_secret.data.url}
          basic_auth:
            username: ${kubernetes_secret.grafana_cloud_secret.data.username}
            password: ${kubernetes_secret.grafana_cloud_secret.data.password}

      scrape_configs:
        - job_name: kubernetes-pods
          kubernetes_sd_configs:
            - role: pod
          relabel_configs:
            - source_labels: [__meta_kubernetes_pod_controller_name]
              regex: ([0-9a-z-.]+?)(-[0-9a-f]{8,10})?
              action: replace
              target_label: __tmp_controller_name
            - source_labels: [__meta_kubernetes_pod_label_app_kubernetes_io_name, __meta_kubernetes_pod_label_app, __tmp_controller_name, __meta_kubernetes_pod_name]
              regex: ^;*([^;]+)(;.*)?$
              action: replace
              target_label: app
            - source_labels: [__meta_kubernetes_pod_label_app_kubernetes_io_component, __meta_kubernetes_pod_label_component]
              regex: ^;*([^;]+)(;.*)?$
              action: replace
              target_label: component
            - action: replace
              source_labels:
              - __meta_kubernetes_pod_node_name
              target_label: node_name
            - action: replace
              source_labels:
              - __meta_kubernetes_namespace
              target_label: namespace
            - action: replace
              replacement: $1
              separator: /
              source_labels:
              - namespace
              - app
              target_label: job
            - action: replace
              source_labels:
              - __meta_kubernetes_pod_name
              target_label: pod
            - action: replace
              source_labels:
              - __meta_kubernetes_pod_container_name
              target_label: container
            - action: replace
              replacement: /var/log/pods/*$1/*.log
              separator: /
              source_labels:
              - __meta_kubernetes_pod_uid
              - __meta_kubernetes_pod_container_name
              target_label: __path__
            - action: replace
              source_labels:
              - __meta_kubernetes_pod_label_region
              target_label: region
            - action: replace
              source_labels:
              - __meta_kubernetes_pod_label_env
              target_label: env
            - action: replace
              source_labels:
              - __meta_kubernetes_pod_label_cloud
              target_label: cloud
            - action: replace
              source_labels:
              - __meta_kubernetes_pod_label_version
              target_label: version
          pipeline_stages:
            - json:
                expressions:
                  timestamp: timestamp
                  level: level
                  message: message
                  logger: logger
                  traceID: trace_id
            - labels:
                level:
                logger:
                traceID:
    EOT
  }
}

# Promtail DaemonSet
resource "kubernetes_daemonset" "promtail" {
  metadata {
    name      = "promtail"
    namespace = kubernetes_namespace.monitoring.metadata[0].name
    labels = {
      app = "promtail"
    }
  }

  spec {
    selector {
      match_labels = {
        app = "promtail"
      }
    }

    template {
      metadata {
        labels = {
          app = "promtail"
        }
      }

      spec {
        service_account_name = kubernetes_service_account.promtail.metadata[0].name

        volume {
          name = "config"
          config_map {
            name = kubernetes_config_map.promtail_config.metadata[0].name
          }
        }

        volume {
          name = "run"
          empty_dir {}
        }

        volume {
          name = "pods"
          host_path {
            path = "/var/log/pods"
          }
        }

        volume {
          name = "containers"
          host_path {
            path = "/var/lib/docker/containers"
          }
        }

        container {
          name  = "promtail"
          image = "grafana/promtail:2.9.2"

          args = [
            "-config.file=/etc/promtail/promtail.yaml",
          ]

          port {
            container_port = 9080
            name           = "http-metrics"
          }

          env {
            name = "HOSTNAME"
            value_from {
              field_ref {
                field_path = "spec.nodeName"
              }
            }
          }

          volume_mount {
            name       = "config"
            mount_path = "/etc/promtail"
          }

          volume_mount {
            name       = "run"
            mount_path = "/run/promtail"
          }

          volume_mount {
            name       = "pods"
            mount_path = "/var/log/pods"
            read_only  = true
          }

          volume_mount {
            name       = "containers"
            mount_path = "/var/lib/docker/containers"
            read_only  = true
          }

          liveness_probe {
            http_get {
              path = "/ready"
              port = "http-metrics"
            }
            initial_delay_seconds = 10
            timeout_seconds       = 1
          }

          readiness_probe {
            http_get {
              path = "/ready"
              port = "http-metrics"
            }
            initial_delay_seconds = 10
            timeout_seconds       = 1
          }

          security_context {
            privileged  = false
            run_as_user = 0
          }
        }
      }
    }
  }
}

#
# nocheckin :Infra! :Robustness!: proper monitoring with OLTP metrics/spans/logs/alerts (in one place?)
#  (Prometheus/Grafana? Honeycomb? Signoz?)
# 
