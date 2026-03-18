use std::path::PathBuf;

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::merge_metadata;
use super::component::{StackComponentJson, StackComponentOptions};
use super::config::{StackConfigJson, StackConfigOptions};
use super::domain::{StackDomainJson, StackDomainOptions};
use super::environment::{StackEnvironmentJson, StackEnvironmentOptions};
use super::ingress::{StackIngressJson, StackIngressOptions};
use super::network::{StackNetworkJson, StackNetworkOptions};
use super::secret::{StackSecretJson, StackSecretOptions};
use super::service::{StackServiceJson, StackServiceOptions};
use super::volume::{StackVolumeJson, StackVolumeOptions};
use super::workload::{StackWorkloadJson, StackWorkloadOptions};

/// Destack stack configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackOptions {
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Source-backed stack entry file.
    pub entry: Option<PathBuf>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Target references used by this stack.
    pub targets: Vec<String>,
    /// Typed component instances that lower into the stack graph.
    pub components: IndexMap<String, StackComponentOptions>,
    /// Named deployable workloads.
    pub workloads: IndexMap<String, StackWorkloadOptions>,
    /// Named services.
    pub services: IndexMap<String, StackServiceOptions>,
    /// Named attached volumes.
    pub volumes: IndexMap<String, StackVolumeOptions>,
    /// Named attached configs.
    pub configs: IndexMap<String, StackConfigOptions>,
    /// Named secret references.
    pub secrets: IndexMap<String, StackSecretOptions>,
    /// Named domains and DNS ownership.
    pub domains: IndexMap<String, StackDomainOptions>,
    /// External traffic and asset ingress.
    pub ingress: StackIngressOptions,
    /// Internal network topology settings.
    pub network: StackNetworkOptions,
    /// Environment overlays for this stack.
    pub environments: IndexMap<String, StackEnvironmentOptions>,
}

impl StackOptions {
    /// Inherit unset stack settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);

        if self.entry.is_none() {
            self.entry = parent.entry.clone();
        }
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }
        if self.targets.is_empty() {
            self.targets = parent.targets.clone();
        }

        for (name, component) in &parent.components {
            if let Some(current) = self.components.get_mut(name) {
                current.extend_from(component);
            } else {
                self.components.insert(name.clone(), component.clone());
            }
        }

        for (name, workload) in &parent.workloads {
            if let Some(current) = self.workloads.get_mut(name) {
                current.extend_from(workload);
            } else {
                self.workloads.insert(name.clone(), workload.clone());
            }
        }

        for (name, service) in &parent.services {
            if let Some(current) = self.services.get_mut(name) {
                current.extend_from(service);
            } else {
                self.services.insert(name.clone(), service.clone());
            }
        }

        for (name, volume) in &parent.volumes {
            if let Some(current) = self.volumes.get_mut(name) {
                current.extend_from(volume);
            } else {
                self.volumes.insert(name.clone(), volume.clone());
            }
        }

        for (name, config) in &parent.configs {
            if let Some(current) = self.configs.get_mut(name) {
                current.extend_from(config);
            } else {
                self.configs.insert(name.clone(), config.clone());
            }
        }

        for (name, secret) in &parent.secrets {
            if let Some(current) = self.secrets.get_mut(name) {
                current.extend_from(secret);
            } else {
                self.secrets.insert(name.clone(), secret.clone());
            }
        }

        for (name, domain) in &parent.domains {
            if let Some(current) = self.domains.get_mut(name) {
                current.extend_from(domain);
            } else {
                self.domains.insert(name.clone(), domain.clone());
            }
        }

        self.ingress.extend_from(&parent.ingress);
        self.network.extend_from(&parent.network);

        for (name, environment) in &parent.environments {
            if let Some(current) = self.environments.get_mut(name) {
                current.extend_from(environment);
            } else {
                self.environments.insert(name.clone(), environment.clone());
            }
        }
    }
}

impl From<&StackJson> for StackOptions {
    fn from(json: &StackJson) -> Self {
        Self {
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            entry: json.entry.as_ref().map(PathBuf::from),
            provider: json.provider.clone(),
            targets: json.targets.clone().unwrap_or_default(),
            components: json
                .components
                .as_ref()
                .map(|components| {
                    components
                        .iter()
                        .map(|(name, component)| {
                            (name.clone(), StackComponentOptions::from(component))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            workloads: json
                .workloads
                .as_ref()
                .map(|workloads| {
                    workloads
                        .iter()
                        .map(|(name, workload)| {
                            (name.clone(), StackWorkloadOptions::from(workload))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            services: json
                .services
                .as_ref()
                .map(|services| {
                    services
                        .iter()
                        .map(|(name, service)| (name.clone(), StackServiceOptions::from(service)))
                        .collect()
                })
                .unwrap_or_default(),
            volumes: json
                .volumes
                .as_ref()
                .map(|volumes| {
                    volumes
                        .iter()
                        .map(|(name, volume)| (name.clone(), StackVolumeOptions::from(volume)))
                        .collect()
                })
                .unwrap_or_default(),
            configs: json
                .configs
                .as_ref()
                .map(|configs| {
                    configs
                        .iter()
                        .map(|(name, config)| (name.clone(), StackConfigOptions::from(config)))
                        .collect()
                })
                .unwrap_or_default(),
            secrets: json
                .secrets
                .as_ref()
                .map(|secrets| {
                    secrets
                        .iter()
                        .map(|(name, secret)| (name.clone(), StackSecretOptions::from(secret)))
                        .collect()
                })
                .unwrap_or_default(),
            domains: json
                .domains
                .as_ref()
                .map(|domains| {
                    domains
                        .iter()
                        .map(|(name, domain)| (name.clone(), StackDomainOptions::from(domain)))
                        .collect()
                })
                .unwrap_or_default(),
            ingress: StackIngressOptions::from(&json.ingress),
            network: StackNetworkOptions::from(&json.network),
            environments: json
                .environments
                .as_ref()
                .map(|environments| {
                    environments
                        .iter()
                        .map(|(name, environment)| {
                            (name.clone(), StackEnvironmentOptions::from(environment))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// A deployment topology node.
///
/// Inputs: targets, components, explicit graph nodes, and environment overlays.
/// Outputs: one normalized stack graph.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackJson {
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Source-backed stack entry file.
    pub entry: Option<String>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Target references used by this stack.
    pub targets: Option<Vec<String>>,
    /// Typed component instances that lower into the stack graph.
    pub components: Option<IndexMap<String, StackComponentJson>>,
    /// Named deployable workloads.
    pub workloads: Option<IndexMap<String, StackWorkloadJson>>,
    /// Named services.
    pub services: Option<IndexMap<String, StackServiceJson>>,
    /// Named attached volumes.
    pub volumes: Option<IndexMap<String, StackVolumeJson>>,
    /// Named attached configs.
    pub configs: Option<IndexMap<String, StackConfigJson>>,
    /// Named secret references.
    pub secrets: Option<IndexMap<String, StackSecretJson>>,
    /// Named domains and DNS ownership.
    pub domains: Option<IndexMap<String, StackDomainJson>>,
    /// External traffic and asset ingress.
    #[serde(default)]
    pub ingress: StackIngressJson,
    /// Internal network topology settings.
    #[serde(default)]
    pub network: StackNetworkJson,
    /// Environment overlays for this stack.
    pub environments: Option<IndexMap<String, StackEnvironmentJson>>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::{
        StackComponentJson, StackEnvironmentOptions, StackRunKind, StackServiceJson,
        StackWorkloadJson, StackWorkloadOptions,
    };
    use super::{StackEnvironmentJson, StackJson, StackOptions};

    /// Parse one full stack graph with workloads, services, volumes, configs, secrets, domains, and ingress.
    #[test]
    fn test_stack_options_parse_full_stack_shape() {
        let json: StackJson = serde_json::from_value(json!({
            "entry": "./infra/production.ds",
            "provider": {
                "cloudflare": {
                    "smartPlacement": true
                }
            },
            "targets": ["web", "api"],
            "components": {
                "client": {
                    "type": "destack.sh/component/client",
                    "version": "1",
                    "with": {
                        "target": "web",
                        "service": "api",
                        "serviceEnv": "PUBLIC_API_ORIGIN",
                        "domain": "example.com"
                    }
                }
            },
            "volumes": {
                "data": {
                    "size": "100Gi",
                    "class": "fast"
                }
            },
            "configs": {
                "app": {
                    "data": {
                        "LOG_LEVEL": "info"
                    }
                }
            },
            "secrets": {
                "dbPassword": {
                    "reference": "production/database/password"
                }
            },
            "domains": {
                "primary": {
                    "name": "example.com",
                    "mode": "managed",
                    "dns": {
                        "provider": "cloudflare",
                        "account": "cloudflareMain"
                    }
                }
            },
            "workloads": {
                "api": {
                    "target": "api",
                    "run": {
                        "kind": "http",
                        "mount": "/api"
                    },
                    "bindings": {
                        "database": {
                            "service": "db",
                            "field": "url"
                        },
                        "password": {
                            "secret": "dbPassword"
                        }
                    },
                    "mounts": {
                        "data": {
                            "volume": "data",
                            "path": "/data"
                        }
                    },
                    "env": {
                        "DATABASE_URL": {
                            "binding": "database"
                        }
                    }
                }
            },
            "services": {
                "api": {
                    "type": "destack.sh/service/http",
                    "workloads": ["api"],
                    "protocol": "http",
                    "port": 443,
                    "targetPort": 3000,
                    "access": {
                        "mode": "required",
                        "issuer": {
                            "service": "universe"
                        },
                        "audiences": ["internal"],
                        "callers": ["web", "worker"]
                    }
                },
                "db": {
                    "type": "destack.sh/service/database",
                    "protocol": "postgres",
                    "url": "postgres://db.internal:5432/app"
                }
            },
            "ingress": {
                "routes": {
                    "api": {
                        "domain": "primary",
                        "match": "/api/:path*",
                        "service": "api",
                        "methods": ["GET", "POST"],
                        "access": {
                            "mode": "required",
                            "issuer": {
                                "service": "universe"
                            },
                            "audiences": ["app"],
                            "scopes": ["user"],
                            "roles": ["member"]
                        }
                    }
                }
            },
            "network": {
                "callbacks": ["https://app.example.com/callback"],
                "trustedOrigins": ["https://example.com"],
                "tlsMode": "automatic"
            }
        }))
        .expect("stack json should parse");

        let options = StackOptions::from(&json);

        assert_eq!(options.targets, vec!["web".to_string(), "api".to_string()]);
        assert!(options.provider.is_some());
        assert!(options.components.contains_key("client"));
        assert!(options.volumes.contains_key("data"));
        assert!(options.configs.contains_key("app"));
        assert!(options.secrets.contains_key("dbPassword"));
        assert!(options.domains.contains_key("primary"));
        assert!(options.workloads.contains_key("api"));
        assert!(options.services.contains_key("api"));
        assert_eq!(options.ingress.routes.len(), 1);
        assert_eq!(
            options
                .ingress
                .routes
                .get("api")
                .and_then(|route| route.domain.as_deref()),
            Some("primary")
        );
        assert_eq!(
            options
                .ingress
                .routes
                .get("api")
                .and_then(|route| route.service.as_deref()),
            Some("api")
        );
        assert_eq!(
            options
                .services
                .get("api")
                .and_then(|service| service.access.mode),
            Some(super::super::StackAccessMode::Required)
        );
        assert_eq!(
            options
                .ingress
                .routes
                .get("api")
                .and_then(|route| route.access.issuer.as_ref())
                .and_then(|issuer| issuer.service.as_deref()),
            Some("universe")
        );
    }

    /// Inherit unset component fields from one parent component.
    #[test]
    fn test_stack_component_extend_inherits_missing_fields() {
        let parent: StackComponentJson = serde_json::from_value(json!({
            "type": "destack.sh/component/client",
            "version": "1",
            "with": {
                "target": "web"
            }
        }))
        .expect("parent component should parse");
        let child: StackComponentJson = serde_json::from_value(json!({
            "labels": {
                "destack.sh/component": "app"
            }
        }))
        .expect("child component should parse");

        let mut child = super::super::StackComponentOptions::from(&child);
        let parent = super::super::StackComponentOptions::from(&parent);
        child.extend_from(&parent);

        assert_eq!(child.r#type.as_deref(), Some("destack.sh/component/client"));
        assert_eq!(child.version.as_deref(), Some("1"));
        assert!(child.with.is_some());
        assert_eq!(
            child.labels.get("destack.sh/component").map(String::as_str),
            Some("app")
        );
    }

    /// Inherit unset workload fields from one parent workload.
    #[test]
    fn test_stack_workload_extend_inherits_missing_fields() {
        let parent: StackWorkloadJson = serde_json::from_value(json!({
            "target": "api",
            "run": {
                "kind": "http"
            },
            "capacity": {
                "limits": {
                    "memory": "1Gi"
                }
            }
        }))
        .expect("parent workload should parse");
        let child: StackWorkloadJson = serde_json::from_value(json!({
            "capacity": {
                "requests": {
                    "memory": "256Mi"
                }
            }
        }))
        .expect("child workload should parse");

        let mut child = StackWorkloadOptions::from(&child);
        let parent = StackWorkloadOptions::from(&parent);
        child.extend_from(&parent);

        assert_eq!(child.target.as_deref(), Some("api"));
        assert_eq!(child.run.kind, Some(StackRunKind::Http));
        assert_eq!(child.capacity.requests.memory.as_deref(), Some("256Mi"));
        assert_eq!(child.capacity.limits.memory.as_deref(), Some("1Gi"));
    }

    /// Inherit unset service fields from one parent service.
    #[test]
    fn test_stack_service_extend_inherits_missing_fields() {
        let parent: StackServiceJson = serde_json::from_value(json!({
            "type": "destack.sh/service/http",
            "workloads": ["api"],
            "protocol": "http",
            "port": 443,
            "targetPort": 3000
        }))
        .expect("parent service should parse");
        let child: StackServiceJson = serde_json::from_value(json!({
            "url": "https://api.example.com"
        }))
        .expect("child service should parse");

        let mut child = super::super::StackServiceOptions::from(&child);
        let parent = super::super::StackServiceOptions::from(&parent);
        child.extend_from(&parent);

        assert_eq!(child.r#type.as_deref(), Some("destack.sh/service/http"));
        assert_eq!(child.workloads, vec!["api"]);
        assert_eq!(child.protocol.as_deref(), Some("http"));
        assert_eq!(child.port, Some(443));
        assert_eq!(child.target_port, Some(3000));
        assert_eq!(child.url.as_deref(), Some("https://api.example.com"));
    }

    /// Inherit unset service access fields from one parent service.
    #[test]
    fn test_stack_service_extend_inherits_missing_access() {
        let parent: StackServiceJson = serde_json::from_value(json!({
            "workloads": ["api"],
            "access": {
                "mode": "required",
                "issuer": {
                    "service": "universe"
                },
                "audiences": ["internal"],
                "callers": ["web"]
            }
        }))
        .expect("parent service should parse");
        let child: StackServiceJson = serde_json::from_value(json!({
            "url": "https://api.example.com",
            "access": {
                "issuer": {
                    "url": "https://auth.example.com"
                },
                "callers": ["worker"]
            }
        }))
        .expect("child service should parse");

        let mut child = super::super::StackServiceOptions::from(&child);
        let parent = super::super::StackServiceOptions::from(&parent);
        child.extend_from(&parent);

        assert_eq!(
            child.access.mode,
            Some(super::super::StackAccessMode::Required)
        );
        assert_eq!(
            child
                .access
                .issuer
                .as_ref()
                .and_then(|issuer| issuer.service.as_deref()),
            Some("universe")
        );
        assert_eq!(
            child
                .access
                .issuer
                .as_ref()
                .and_then(|issuer| issuer.url.as_deref()),
            Some("https://auth.example.com")
        );
        assert_eq!(child.access.audiences, vec!["internal".to_string()]);
        assert_eq!(child.access.callers, vec!["worker".to_string()]);
    }

    /// Inherit unset environment fields from one parent environment.
    #[test]
    fn test_stack_environment_extend_inherits_missing_fields() {
        let parent: StackEnvironmentJson = serde_json::from_value(json!({
            "ephemeral": true,
            "components": {
                "worker": {
                    "type": "destack.sh/component/worker"
                }
            },
            "network": {
                "trustedOrigins": ["https://example.com"]
            }
        }))
        .expect("parent environment should parse");
        let child: StackEnvironmentJson = serde_json::from_value(json!({
            "labels": {
                "destack.sh/environment": "dev"
            }
        }))
        .expect("child environment should parse");

        let mut child = StackEnvironmentOptions::from(&child);
        let parent = StackEnvironmentOptions::from(&parent);
        child.extend_from(&parent);

        assert_eq!(
            child
                .labels
                .get("destack.sh/environment")
                .map(String::as_str),
            Some("dev")
        );
        assert_eq!(child.ephemeral, Some(true));
        assert!(child.components.contains_key("worker"));
        assert_eq!(child.network.trusted_origins, vec!["https://example.com"]);
    }

    /// Parse consumer delivery and scheduled trigger settings.
    #[test]
    fn test_stack_workload_parse_run_delivery_and_trigger() {
        let consumer: StackWorkloadJson = serde_json::from_value(json!({
            "target": "worker",
            "run": {
                "kind": "consumer",
                "trigger": {
                    "kind": "destack.sh/trigger/queue",
                    "service": "emailQueue"
                },
                "delivery": {
                    "batch": {
                        "maxSize": 100,
                        "maxWaitSeconds": 5
                    },
                    "retry": {
                        "maxAttempts": 5,
                        "backoffSeconds": 30
                    },
                    "visibilityTimeoutSeconds": 300
                }
            },
            "health": {
                "readiness": {
                    "path": "/readyz",
                    "method": "GET",
                    "port": 8080,
                    "initialDelaySeconds": 5,
                    "successThreshold": 2
                }
            },
            "placement": {
                "regions": ["iad1"],
                "affinity": ["pool:workers"],
                "antiAffinity": ["host"],
                "spread": ["zone"]
            },
            "identity": {
                "name": "emails",
                "audiences": ["destack", "queue"],
                "provider": {
                    "aws": {
                        "role": "emails"
                    }
                }
            },
            "timeouts": {
                "job": "10m",
                "operation": "30s"
            },
            "runtime": {
                "scheduler": {
                    "maxTasks": 64
                },
                "heap": {
                    "maxBytes": 1048576
                }
            }
        }))
        .expect("consumer workload should parse");
        let schedule: StackWorkloadJson = serde_json::from_value(json!({
            "target": "worker",
            "run": {
                "kind": "schedule",
                "trigger": {
                    "kind": "destack.sh/trigger/schedule",
                    "schedule": "0 * * * *"
                }
            }
        }))
        .expect("schedule workload should parse");

        let consumer = StackWorkloadOptions::from(&consumer);
        let schedule = StackWorkloadOptions::from(&schedule);

        assert_eq!(consumer.run.kind, Some(StackRunKind::Consumer));
        assert_eq!(
            consumer.run.trigger.kind.as_deref(),
            Some("destack.sh/trigger/queue")
        );
        assert_eq!(consumer.run.trigger.service.as_deref(), Some("emailQueue"));
        assert_eq!(consumer.run.delivery.batch.max_size, Some(100));
        assert_eq!(consumer.run.delivery.retry.max_attempts, Some(5));
        assert_eq!(consumer.health.readiness.method.as_deref(), Some("GET"));
        assert_eq!(consumer.health.readiness.port, Some(8080));
        assert_eq!(consumer.health.readiness.initial_delay_seconds, Some(5));
        assert_eq!(consumer.health.readiness.success_threshold, Some(2));
        assert_eq!(
            consumer.placement.affinity,
            vec!["pool:workers".to_string()]
        );
        assert_eq!(consumer.placement.anti_affinity, vec!["host".to_string()]);
        assert_eq!(consumer.placement.spread, vec!["zone".to_string()]);
        assert_eq!(consumer.identity.name.as_deref(), Some("emails"));
        assert_eq!(
            consumer.identity.audiences,
            vec!["destack".to_string(), "queue".to_string()]
        );
        assert_eq!(consumer.timeouts.job.as_deref(), Some("10m"));
        assert_eq!(consumer.timeouts.operation.as_deref(), Some("30s"));
        let runtime = consumer
            .effective_runtime_overrides()
            .expect("effective runtime overrides should exist");

        assert_eq!(
            runtime
                .scheduler
                .as_ref()
                .and_then(|scheduler| scheduler.max_tasks),
            Some(64)
        );
        assert_eq!(
            runtime.heap.as_ref().and_then(|heap| heap.max_bytes),
            Some(1048576)
        );
        assert_eq!(
            schedule.run.trigger.kind.as_deref(),
            Some("destack.sh/trigger/schedule")
        );
        assert_eq!(schedule.run.trigger.schedule.as_deref(), Some("0 * * * *"));
    }

    /// Inherit unset workload timeout fields from one parent workload.
    #[test]
    fn test_stack_workload_extend_inherits_missing_timeouts() {
        let parent: StackWorkloadJson = serde_json::from_value(json!({
            "timeouts": {
                "startup": "30s",
                "shutdown": "15s"
            }
        }))
        .expect("parent workload should parse");
        let child: StackWorkloadJson = serde_json::from_value(json!({
            "timeouts": {
                "request": "60s"
            }
        }))
        .expect("child workload should parse");

        let mut child = StackWorkloadOptions::from(&child);
        let parent = StackWorkloadOptions::from(&parent);
        child.extend_from(&parent);

        assert_eq!(child.timeouts.startup.as_deref(), Some("30s"));
        assert_eq!(child.timeouts.request.as_deref(), Some("60s"));
        assert_eq!(child.timeouts.shutdown.as_deref(), Some("15s"));
    }

    /// Inherit unset probe, placement, and identity fields from one parent workload.
    #[test]
    fn test_stack_workload_extend_inherits_probe_placement_and_identity() {
        let parent: StackWorkloadJson = serde_json::from_value(json!({
            "health": {
                "startup": {
                    "path": "/startup",
                    "method": "GET",
                    "port": 8080,
                    "initialDelaySeconds": 10,
                    "successThreshold": 2
                }
            },
            "placement": {
                "regions": ["iad1"],
                "affinity": ["pool:blue"],
                "spread": ["zone"]
            },
            "identity": {
                "name": "api",
                "audiences": ["destack"]
            }
        }))
        .expect("parent workload should parse");
        let child: StackWorkloadJson = serde_json::from_value(json!({
            "health": {
                "startup": {
                    "timeoutSeconds": 5
                }
            },
            "placement": {
                "antiAffinity": ["host"]
            },
            "identity": {
                "provider": {
                    "gcp": {
                        "serviceAccount": "api"
                    }
                }
            }
        }))
        .expect("child workload should parse");

        let mut child = StackWorkloadOptions::from(&child);
        let parent = StackWorkloadOptions::from(&parent);
        child.extend_from(&parent);

        assert_eq!(child.health.startup.path.as_deref(), Some("/startup"));
        assert_eq!(child.health.startup.method.as_deref(), Some("GET"));
        assert_eq!(child.health.startup.port, Some(8080));
        assert_eq!(child.health.startup.initial_delay_seconds, Some(10));
        assert_eq!(child.health.startup.success_threshold, Some(2));
        assert_eq!(child.health.startup.timeout_seconds, Some(5));
        assert_eq!(child.placement.regions, vec!["iad1".to_string()]);
        assert_eq!(child.placement.affinity, vec!["pool:blue".to_string()]);
        assert_eq!(child.placement.anti_affinity, vec!["host".to_string()]);
        assert_eq!(child.placement.spread, vec!["zone".to_string()]);
        assert_eq!(child.identity.name.as_deref(), Some("api"));
        assert_eq!(child.identity.audiences, vec!["destack".to_string()]);
        assert!(child.identity.provider.is_some());
    }

    /// Merge named ingress rules incrementally by identity.
    #[test]
    fn test_stack_ingress_extend_merges_named_rules() {
        let parent: StackJson = serde_json::from_value(json!({
            "ingress": {
                "routes": {
                    "site": {
                        "domain": "primary",
                        "match": "/",
                        "service": "site"
                    }
                }
            }
        }))
        .expect("parent stack should parse");
        let child: StackJson = serde_json::from_value(json!({
            "ingress": {
                "routes": {
                    "api": {
                        "domain": "primary",
                        "match": "/api/:path*",
                        "service": "api"
                    }
                }
            }
        }))
        .expect("child stack should parse");

        let parent = StackOptions::from(&parent);
        let mut child = StackOptions::from(&child);
        child.extend_from(&parent);

        assert!(child.ingress.routes.contains_key("site"));
        assert!(child.ingress.routes.contains_key("api"));
        assert_eq!(
            child
                .ingress
                .routes
                .get("site")
                .and_then(|route| route.domain.as_deref()),
            Some("primary")
        );
        assert_eq!(
            child
                .ingress
                .routes
                .get("site")
                .and_then(|route| route.service.as_deref()),
            Some("site")
        );
    }

    /// Inherit unset route access fields from one parent route.
    #[test]
    fn test_stack_ingress_extend_inherits_route_access() {
        let parent: StackJson = serde_json::from_value(json!({
            "ingress": {
                "routes": {
                    "app": {
                        "match": "/app/:path*",
                        "service": "app",
                        "access": {
                            "mode": "required",
                            "issuer": {
                                "service": "universe"
                            },
                            "audiences": ["app"],
                            "scopes": ["user"]
                        }
                    }
                }
            }
        }))
        .expect("parent stack should parse");
        let child: StackJson = serde_json::from_value(json!({
            "ingress": {
                "routes": {
                    "app": {
                        "access": {
                            "issuer": {
                                "url": "https://auth.example.com"
                            },
                            "roles": ["member"]
                        }
                    }
                }
            }
        }))
        .expect("child stack should parse");

        let parent = StackOptions::from(&parent);
        let mut child = StackOptions::from(&child);
        child.extend_from(&parent);

        let route = child.ingress.routes.get("app").expect("route should exist");

        assert_eq!(
            route.access.mode,
            Some(super::super::StackAccessMode::Required)
        );
        assert_eq!(
            route
                .access
                .issuer
                .as_ref()
                .and_then(|issuer| issuer.service.as_deref()),
            Some("universe")
        );
        assert_eq!(
            route
                .access
                .issuer
                .as_ref()
                .and_then(|issuer| issuer.url.as_deref()),
            Some("https://auth.example.com")
        );
        assert_eq!(route.access.audiences, vec!["app".to_string()]);
        assert_eq!(route.access.scopes, vec!["user".to_string()]);
        assert_eq!(route.access.roles, vec!["member".to_string()]);
    }

    /// Parse route policy and volume durability settings.
    #[test]
    fn test_stack_parse_route_policy_and_volume_durability() {
        let json: StackJson = serde_json::from_value(json!({
            "volumes": {
                "uploads": {
                    "size": "500Gi",
                    "class": "ssd",
                    "access": "readWriteMany",
                    "retention": "retain",
                    "backup": {
                        "enabled": true,
                        "schedule": "0 2 * * *",
                        "retentionDays": 30
                    }
                }
            },
            "ingress": {
                "routes": {
                    "app": {
                        "domain": "primary",
                        "match": "/:path*",
                        "service": "app",
                        "policy": {
                            "timeout": "30s",
                            "retries": {
                                "attempts": 3,
                                "perTryTimeout": "5s",
                                "conditions": ["5xx", "gateway-error"]
                            },
                            "cache": {
                                "enabled": true,
                                "ttl": "1h",
                                "staleWhileRevalidate": "5m",
                                "vary": ["accept-encoding"]
                            }
                        }
                    }
                }
            }
        }))
        .expect("stack json should parse");

        let options = StackOptions::from(&json);
        let uploads = options.volumes.get("uploads").expect("volume should exist");
        let route = options
            .ingress
            .routes
            .get("app")
            .expect("route should exist");

        assert_eq!(uploads.access.as_deref(), Some("readWriteMany"));
        assert_eq!(uploads.retention.as_deref(), Some("retain"));
        assert_eq!(uploads.backup.enabled, Some(true));
        assert_eq!(uploads.backup.retention_days, Some(30));
        assert_eq!(route.policy.timeout.as_deref(), Some("30s"));
        assert_eq!(route.policy.retries.attempts, Some(3));
        assert_eq!(route.policy.cache.enabled, Some(true));
        assert_eq!(route.policy.cache.ttl.as_deref(), Some("1h"));
    }
}
