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
          image = "k8s.gcr.io/metrics-server/metrics-server:v0.6.1"

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
    api_groups = [""]
    resources  = ["*"]
    verbs      = ["get", "list", "watch"]
  }
  rule {
    api_groups = ["apps"]
    resources  = ["*"]
    verbs      = ["get", "list", "watch"]
  }
  rule {
    api_groups = ["extensions"]
    resources  = ["*"]
    verbs      = ["get", "list", "watch"]
  }
  rule {
    api_groups = ["batch"]
    resources  = ["*"]
    verbs      = ["get", "list", "watch"]
  }
  rule {
    api_groups = ["autoscaling"]
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
          image = "quay.io/coreos/kube-state-metrics:v1.9.8"

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
# TODO :Infra: proper monitoring with prometheus, OLTP, ...s
# 
