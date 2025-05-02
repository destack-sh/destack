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
# Jaeger
# 

resource "kubernetes_deployment" "jaeger" {
  metadata {
    name = "jaeger"
    labels = {
      app = "jaeger"
    }
    namespace = "monitoring"
  }

  spec {
    replicas = 1

    selector {
      match_labels = {
        app = "jaeger"
      }
    }

    template {
      metadata {
        labels = {
          app = "jaeger"
        }
      }

      spec {
        container {
          image = "jaegertracing/all-in-one:latest"
          name  = "jaeger"

          port {
            container_port = 16686
          }
          port {
            container_port = 4317
          }
          port {
            container_port = 4318
          }
        }
      }
    }
  }
}

resource "kubernetes_service" "jaeger" {
  metadata {
    name      = "jaeger"
    namespace = "monitoring"
  }
  spec {
    selector = {
      app = "jaeger"
    }
    port {
      port = 16686
      name = "ui"
    }
    port {
      port = 4317
      name = "grpc"
    }
    type = "ClusterIP"
  }
}

#
# nocheckin :Infra! :Robustness!: proper monitoring with OLTP metrics/spans/logs/alerts (in one place?)
#  (Prometheus/Grafana? Honeycomb? Signoz?)
# 
