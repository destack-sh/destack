import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as eks from "@pulumi/eks";
import * as k8s from "@pulumi/kubernetes";
import * as pulumi from "@pulumi/pulumi";
import { getBenchUserS3AccessKey, makeALBController, makeEbsCsiDriver } from "./aws";
import * as random from "@pulumi/random";
import * as fs from "fs";
import * as yaml from "js-yaml";

// configuration
const config = new pulumi.Config();
const minClusterSize = config.getNumber("minClusterSize");
const maxClusterSize = config.getNumber("maxClusterSize");
const desiredClusterSize = config.getNumber("desiredClusterSize");
const eksNodeInstanceType = config.get("eksNodeInstanceType");
const vpcNetworkCidr = config.get("vpcNetworkCidr");

// VPC
const eksVpc = new awsx.ec2.Vpc("eks-vpc", {
  enableDnsHostnames: true,
  cidrBlock: vpcNetworkCidr,
  subnetSpecs: [
    { type: "Public", tags: { "kubernetes.io/role/elb": "1" } },
    { type: "Private", tags: { "kubernetes.io/role/internal-elb": "1" } },
  ],
});

// EKS cluster
const eksCluster = new eks.Cluster("eks-cluster", {
  name: "bench-" + config.require("env"),
  vpcId: eksVpc.vpcId,
  // public subnets for load balancers
  publicSubnetIds: eksVpc.publicSubnetIds,
  // private subnets for cluster nodes
  privateSubnetIds: eksVpc.privateSubnetIds,
  instanceType: eksNodeInstanceType,
  desiredCapacity: desiredClusterSize,
  minSize: minClusterSize,
  maxSize: maxClusterSize,
  nodeAssociatePublicIpAddress: false,
});

// image pull secrets for GHCR
const ghrcToken = config.requireSecret("ghcrToken");
const imagePullSecret = new k8s.core.v1.Secret(
  "image-pull-secret",
  {
    metadata: { namespace: "default" },
    type: "kubernetes.io/dockerconfigjson",
    data: {
      ".dockerconfigjson": ghrcToken.apply((token) =>
        Buffer.from(
          JSON.stringify({
            auths: {
              "ghcr.io": {
                auth: token,
              },
            },
          })
        ).toString("base64")
      ),
    },
  },
  { provider: eksCluster.provider }
);
// AWS base stuff
const albIngressController = makeALBController(eksVpc, eksCluster);
const ebsCsiDriver = makeEbsCsiDriver(eksVpc, eksCluster);

// DB: RDS Aurora Postgres cluster/database
const dbSecurityGroup = new aws.ec2.SecurityGroup("db", {
  // TODO @Cleanup: pods should connect directly to DB instance (not via publicly accessible)
  ingress: [
    {
      fromPort: 5432,
      toPort: 5432,
      protocol: "tcp",
      cidrBlocks: ["0.0.0.0/0"],
    },
  ],
  egress: [
    {
      fromPort: 0,
      toPort: 0,
      protocol: "-1",
      cidrBlocks: ["0.0.0.0/0"],
    },
  ],
});
const globalDb = new aws.rds.Cluster("db", {
  engine: "aurora-postgresql",
  clusterIdentifier: "db",
  engineVersion: "14.6",
  databaseName: "postgres",
  deletionProtection: true,
  masterUsername: "postgres",
  masterPassword: config.requireSecret("dbPassword"),
  backupRetentionPeriod: 7,
  preferredBackupWindow: "04:00-06:00",
  vpcSecurityGroupIds: [dbSecurityGroup.id],
});
const globalDBInstance = new aws.rds.ClusterInstance("db", {
  clusterIdentifier: globalDb.clusterIdentifier,
  instanceClass: "db.t4g.medium",
  engine: "aurora-postgresql",
  engineVersion: "14.6",
  publiclyAccessible: true,
  performanceInsightsEnabled: true,
});
const globalDbSecret = new k8s.core.v1.Secret(
  "db",
  {
    metadata: { namespace: "default" },
    type: "Opaque",
    data: {
      password: config.requireSecret("dbPassword").apply((password) => Buffer.from(password).toString("base64")),
    },
  },
  { provider: eksCluster.provider }
);
// db env vars
const GLOBAL_DB_ENV_VARS = [
  {
    name: "BENCH_DB_NAME",
    value: "postgres",
  },
  {
    name: "BENCH_DB_USER",
    value: "postgres",
  },
  {
    name: "BENCH_DB_PASSWORD",
    valueFrom: {
      secretKeyRef: {
        name: globalDbSecret.metadata.name,
        key: "password",
      },
    },
  },
  {
    name: "BENCH_POSTGRES_HOST",
    value: globalDb.endpoint,
  },
  {
    name: "BENCH_POSTGRES_PORT",
    value: globalDb.port.apply((port) => port.toString()),
  },
  {
    name: "PGCRYPTO_KEY",
    value: config.requireSecret("PGCRYPTO_KEY"),
  },
];

// Search: OpenSearch cluster
const opensearchDomainName = `bench-${config.require("env")}`;
const opensearchSecurityGroup = new aws.ec2.SecurityGroup("opensearch", {
  ingress: [{ fromPort: 443, toPort: 443, protocol: "tcp", cidrBlocks: ["0.0.0.0/0"] }],
  egress: [{ fromPort: 0, toPort: 0, protocol: "-1", cidrBlocks: ["0.0.0.0/0"] }],
  vpcId: eksVpc.vpcId,
});
const opensearchDomain = new aws.opensearch.Domain(opensearchDomainName, {
  domainName: opensearchDomainName,
  engineVersion: "OpenSearch_2.9",
  clusterConfig: {
    instanceType: "t3.medium.search",
    instanceCount: 1,
  },
  domainEndpointOptions: {
    enforceHttps: true,
    tlsSecurityPolicy: "Policy-Min-TLS-1-2-2019-07",
  },
  ebsOptions: {
    ebsEnabled: true,
    volumeSize: 50,
    volumeType: "gp3",
  },
  encryptAtRest: {
    enabled: true,
  },
  nodeToNodeEncryption: {
    enabled: true,
  },
  vpcOptions: {
    subnetIds: eksVpc.privateSubnetIds.apply((ids) => ids.slice(0, 1)),
    securityGroupIds: [opensearchSecurityGroup.id],
  },
  // public access with fine grained access control
  accessPolicies: JSON.stringify({
    Version: "2012-10-17",
    Statement: [
      {
        Effect: "Allow",
        Principal: {
          AWS: "*",
        },
        Action: "es:*",
        Resource: `arn:aws:es:${config.require("awsRegion")}:*:domain/${opensearchDomainName}/*`,
      },
    ],
  }),
  advancedOptions: {
    "rest.action.multi.allow_explicit_index": "true",
  },
  advancedSecurityOptions: {
    enabled: true,
    internalUserDatabaseEnabled: true,
    masterUserOptions: {
      masterUserName: "opensearch",
      masterUserPassword: config.requireSecret("opensearchPassword"),
    },
  },
});
const opensearchSecret = new k8s.core.v1.Secret(
  "opensearch",
  {
    metadata: { namespace: "default" },
    type: "Opaque",
    data: {
      password: config
        .requireSecret("opensearchPassword")
        .apply((password) => Buffer.from(password).toString("base64")),
    },
  },
  { provider: eksCluster.provider }
);
const OPENSEARCH_ENV_VARS = [
  {
    name: "OPENSEARCH_DOMAIN",
    value: opensearchDomainName,
  },
  {
    name: "OPENSEARCH_URL",
    value: opensearchDomain.endpoint,
  },
  {
    name: "OPENSEARCH_PORT",
    value: "443",
  },
  {
    name: "OPENSEARCH_USERNAME",
    value: "opensearch",
  },
  {
    name: "OPENSEARCH_PASSWORD",
    valueFrom: {
      secretKeyRef: {
        name: opensearchSecret.metadata.name,
        key: "password",
      },
    },
  },
];

// Redis: ElasticCache Redis cluster
const redisSecurityGroup = new aws.ec2.SecurityGroup("redis", {
  ingress: [
    {
      fromPort: 6379,
      toPort: 6379,
      protocol: "tcp",
      cidrBlocks: ["0.0.0.0/0"],
    },
  ],
  egress: [
    {
      fromPort: 0,
      toPort: 0,
      protocol: "-1",
      cidrBlocks: ["0.0.0.0/0"],
    },
  ],
  vpcId: eksVpc.vpcId,
});
// connect redis to same VPC as EKS
const redisSubnetGroup = new aws.elasticache.SubnetGroup("redis", {
  subnetIds: eksVpc.privateSubnetIds,
});
// create redis users
// (default user with no password and no access, root user with full access, restricted user with limited access)
const redisRootPassword = new random.RandomPassword("redisRootPassword", {
  length: 32,
  special: false,
});
const redisRootUser = new aws.elasticache.User("redisRootUser", {
  engine: "REDIS",
  accessString: "on ~* +@all",
  userId: "root",
  userName: "root",
  passwords: [redisRootPassword.result],
});
const redisRestrictedPassword = new random.RandomPassword("redisRestrictedPassword", {
  length: 32,
  special: false,
});
const redisWorkerUser = new aws.elasticache.User("redisRestrictedUser", {
  engine: "REDIS",
  // TODO @Security!: don't give worker user full Redis access
  accessString: "on ~* -@all +get +set +ping +incrby +expire +multi +exec",
  userId: "worker",
  userName: "worker",
  passwords: [redisRestrictedPassword.result],
});
// user group
const redisUserGroup = new aws.elasticache.UserGroup("redisUserGroup", {
  engine: "REDIS",
  userGroupId: "redis-user-group",
  userIds: ["default", redisRootUser.userId, redisWorkerUser.userId],
});
// replication group with user ids
const redisReplicationGroup = new aws.elasticache.ReplicationGroup("redis", {
  description: "Redis cluster",
  engine: "redis",
  nodeType: "cache.t3.medium",
  numCacheClusters: 1,
  port: 6379,
  parameterGroupName: "default.redis7",
  securityGroupIds: [redisSecurityGroup.id],
  userGroupIds: [redisUserGroup.userGroupId],
  subnetGroupName: redisSubnetGroup.id,
  transitEncryptionEnabled: true,
});
const REDIS_ROOT_URL = pulumi.interpolate`rediss://${redisRootUser.userName}:${redisRootPassword.result}@${redisReplicationGroup.primaryEndpointAddress}:${redisReplicationGroup.port}`;
const REDIS_WORKER_URL = pulumi.interpolate`rediss://${redisWorkerUser.userName}:${redisRestrictedPassword.result}@${redisReplicationGroup.primaryEndpointAddress}:${redisReplicationGroup.port}`;

// NATS (HELM)
const nats = new k8s.helm.v3.Release("nats", {
  namespace: "default",
  chart: "nats",
  version: "0.19.12",
  repositoryOpts: {
    repo: "https://nats-io.github.io/k8s/helm/charts/",
  },
  values: {
    // disable natbox
    natsbox: {
      enabled: false,
    },
    limits: {
      // 256MB max message size
      maxPayload: 256 * 1024 * 1024,
    },
  },
});

// IAM access to manage bench-user S3 buckets
const s3AccessKey = getBenchUserS3AccessKey();
// put access key (id and secret) in a secret
const s3AccessKeySecret = new k8s.core.v1.Secret("s3AccessKeySecret", {
  metadata: {
    name: "s3-access-key",
  },
  stringData: {
    accessKeyId: s3AccessKey.accessKeyId,
    secretAccessKey: s3AccessKey.secretAccessKey,
  },
  type: "Opaque",
});

// S3 backend env vars
const AWS_BACKEND_ENV_VARS = [
  {
    name: "AWS_REGION",
    value: config.require("awsRegion"),
  },
  {
    name: "AWS_ACCESS_KEY_ID",
    valueFrom: {
      secretKeyRef: {
        name: s3AccessKeySecret.metadata.name,
        key: "accessKeyId",
      },
    },
  },
  {
    name: "AWS_SECRET_ACCESS_KEY",
    valueFrom: {
      secretKeyRef: {
        name: s3AccessKeySecret.metadata.name,
        key: "secretAccessKey",
      },
    },
  },
];

// general backend env vars
const PUBLIC_BACKEND_VARS = [
  {
    name: "ENVIRONMENT",
    value: config.require("env"),
  },
  {
    name: "LOCAL_ENV", // should probably merge this with ENVIRONMENT
    value: config.require("env"),
  },
  {
    name: "NOISY_LOG_LEVEL",
    value: "DEBUG",
  },
  {
    name: "SENTRY_DSN",
    value: config.requireSecret("SENTRY_DSN"),
  },
  {
    name: "NATS_SERVER",
    value: nats.name.apply((name) => `nats://${name}:4222`),
  },
];

const MODEL_PROVIDER_VARS = ["OPENAI_API_KEY", "ANTHROPIC_API_KEY"].map((name) => ({
  name,
  value: config.requireSecret(name),
}));

// public load-balanced API service (also runs internal server)
const apiName = "api";
const apiService = new k8s.core.v1.Service(
  apiName,
  {
    spec: {
      type: "NodePort",
      ports: [{ port: 80, name: "http" }],
      selector: { app: apiName },
    },
  },
  { provider: eksCluster.provider }
);
// internal server service
const serverName = "server";

const version = config.require("version");
// if version is 'current', get the current commit hash
let imageVersion;
if (version == "current") {
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  imageVersion = require("child_process").execSync("git rev-parse --short HEAD").toString().trim();
} else {
  imageVersion = version;
}

// generate kubernetes server roles and such to create deployments
const serverServiceAccount = new k8s.core.v1.ServiceAccount("server-deployment-service-account", {
  metadata: {
    namespace: "default",
  },
});
const serverClusterRole = new k8s.rbac.v1.ClusterRole("server-deployment-cluster-role", {
  rules: [
    {
      apiGroups: [""],
      resources: ["pods", "services", "endpoints", "persistentvolumeclaims", "events", "configmaps", "secrets"],
      verbs: ["get", "watch", "list", "create", "update", "patch", "delete"],
    },
    {
      apiGroups: ["apps"],
      resources: ["deployments", "replicasets"],
      verbs: ["get", "watch", "list", "create", "update", "patch", "delete"],
    },
    {
      apiGroups: ["batch"],
      resources: ["jobs", "cronjobs"],
      verbs: ["get", "watch", "list", "create", "update", "patch", "delete"],
    },
  ],
});
const serverClusterRoleBinding = new k8s.rbac.v1.ClusterRoleBinding("server-deployment-cluster-role-binding", {
  subjects: [
    {
      kind: "ServiceAccount",
      name: serverServiceAccount.metadata.name,
      namespace: "default",
    },
  ],
  roleRef: {
    kind: "ClusterRole",
    name: serverClusterRole.metadata.name,
    apiGroup: "rbac.authorization.k8s.io",
  },
});

const SOCIAL_AUTH_ENV_VARS = [
  "SOCIAL_AUTH_GITHUB_KEY",
  "SOCIAL_AUTH_GITHUB_SECRET",
  "SOCIAL_AUTH_GOOGLE_OAUTH2_KEY",
  "SOCIAL_AUTH_GOOGLE_OAUTH2_SECRET",
].map((name) => ({
  name,
  value: config.requireSecret(name),
}));

const BASE_PRIVATE_BACKEND_ENV_VARS = [
  { name: "LOOPS_API_KEY", value: config.requireSecret("LOOPS_API_KEY") },
  { name: "ALLOWED_HOSTS", value: config.require("apiAllowedHosts") },
  { name: "CORS_ALLOWED_ORIGINS", value: config.require("apiAllowedOrigins") },
  { name: "WEBAPP_URL", value: config.require("webappUrl") },
  { name: "REDIS_URL", value: REDIS_ROOT_URL },
];
const WORKER_ENV_VARS = [
  ...PUBLIC_BACKEND_VARS,
  { name: "REDIS_URL", value: REDIS_WORKER_URL },
  { name: "ALLOW_UNTRUSTED_CODE", value: "true" },
];
// encode as k1=v1;k2=v2;... and then base64
const WORKER_ENV_VARS_ENCODED = pulumi
  .all(WORKER_ENV_VARS.map((env) => pulumi.interpolate`${env.name}=${env.value}`))
  .apply((vars) => Buffer.from(vars.join(";")).toString("base64"));
const KUBERNETES_ENV_VARS = [
  { name: "KUBERNETES_WORKER_IMAGE", value: `ghcr.io/symbolx/bench-worker:${imageVersion}` },
  { name: "KUBERNETES_WORKER_ENV_VARS", value: WORKER_ENV_VARS_ENCODED },
  { name: "KUBERNETES_WORKER_IMAGE_PULL_SECRET_NAME", value: imagePullSecret.metadata.name },
];

// deployment for API service (ASGI Django with Daphne)
const apiDeployment = new k8s.apps.v1.Deployment(
  apiName,
  {
    metadata: { namespace: "default", labels: { app: apiName } },
    spec: {
      replicas: 1,
      selector: { matchLabels: { app: apiName } },
      template: {
        metadata: { labels: { app: apiName }, annotations: { "prometheus.io/scrape": "true" } },
        spec: {
          containers: [
            {
              name: apiName,
              image: `ghcr.io/symbolx/bench-api:${imageVersion}`,
              ports: [{ containerPort: 80, name: "http" }],
              env: [
                ...PUBLIC_BACKEND_VARS,
                ...GLOBAL_DB_ENV_VARS,
                ...OPENSEARCH_ENV_VARS,
                ...AWS_BACKEND_ENV_VARS,
                ...BASE_PRIVATE_BACKEND_ENV_VARS,
                ...SOCIAL_AUTH_ENV_VARS,
              ],
              command: ["sh", "-c"],
              args: ["daphne -b 0.0.0.0 -p 80 bench.asgi:application"],
              resources: { requests: { cpu: "500m", memory: "1000Mi" } },
            },
          ],
          imagePullSecrets: [{ name: imagePullSecret.metadata.name }],
        },
      },
    },
  },
  { provider: eksCluster.provider }
);
// master server for language and orchestration
const serverDeployment = new k8s.apps.v1.Deployment(
  serverName,
  {
    metadata: { namespace: "default", labels: { app: serverName } },
    spec: {
      replicas: 1,
      selector: { matchLabels: { app: serverName } },
      template: {
        metadata: { labels: { app: serverName }, annotations: { "prometheus.io/scrape": "true" } },
        spec: {
          // auto-migrate
          initContainers: [
            {
              name: serverName + "-migrate",
              image: `ghcr.io/symbolx/bench-api:${imageVersion}`,
              env: [
                ...PUBLIC_BACKEND_VARS,
                ...OPENSEARCH_ENV_VARS,
                ...GLOBAL_DB_ENV_VARS,
                ...AWS_BACKEND_ENV_VARS,
                ...BASE_PRIVATE_BACKEND_ENV_VARS,
                { name: "SEND_API_PUB_MSG", value: "" },
              ],
              command: ["/bin/sh", "-c"],
              args: ["python manage.py migrate && python manage.py s3 create && python manage.py libs upsert all"],
            },
          ],
          containers: [
            {
              name: serverName,
              image: `ghcr.io/symbolx/bench-api:${imageVersion}`,
              ports: [{ containerPort: 80, name: "http" }],
              env: [
                ...PUBLIC_BACKEND_VARS,
                ...GLOBAL_DB_ENV_VARS,
                ...OPENSEARCH_ENV_VARS,
                ...MODEL_PROVIDER_VARS,
                ...AWS_BACKEND_ENV_VARS,
                ...BASE_PRIVATE_BACKEND_ENV_VARS,
                ...KUBERNETES_ENV_VARS,
              ],
              command: ["python", "manageserver.py", "all"],
              resources: { requests: { cpu: "1000m", memory: "2000Mi" } },
              readinessProbe: {
                httpGet: { path: "/ready", port: 80 },
                initialDelaySeconds: 15,
                periodSeconds: 10,
              },
              livenessProbe: {
                httpGet: { path: "/healthz", port: 80 },
                initialDelaySeconds: 15,
                periodSeconds: 10,
              },
            },
          ],
          imagePullSecrets: [{ name: imagePullSecret.metadata.name }],
          serviceAccountName: serverServiceAccount.metadata.name,
        },
      },
    },
  },
  { provider: eksCluster.provider }
);

// Expose API service via HTTPS ingress
const apiDomain = "api.bench.is";
// TODO @Infra: manage AWS certificate via aws.acm.Certificate
// (without causing issues with current certificate)
const apiIngress = new k8s.networking.v1.Ingress(
  apiName,
  {
    metadata: {
      annotations: {
        "kubernetes.io/ingress.class": "alb",
        "alb.ingress.kubernetes.io/ssl-redirect": "443",
        "alb.ingress.kubernetes.io/listen-ports": '[{"HTTP": 80}, {"HTTPS":443}]',
        "alb.ingress.kubernetes.io/scheme": "internet-facing",
        // ALB doesn't support cert-manager certs, so we need to provision that cert ACM
        "certificate-arn": "arn:aws:acm:eu-central-1:163349077661:certificate/8271c03a-0830-4c02-81af-b5add9429291",
      },
      namespace: "default",
    },
    spec: {
      tls: [
        {
          hosts: [apiDomain],
          secretName: "api-cert",
        },
      ],
      rules: [
        {
          host: apiDomain,
          http: {
            paths: [
              {
                path: "/",
                pathType: "Prefix",
                backend: {
                  service: {
                    name: apiService.metadata.name,
                    port: { number: 80 },
                  },
                },
              },
            ],
          },
        },
      ],
    },
  },
  { provider: eksCluster.provider }
);

// Monitoring: BetterStack & Prometheus Logs
// see https://betterstack.com/docs/logs/kubernetes#helm
const monitoringNamespace = new k8s.core.v1.Namespace(
  "monitoring",
  { metadata: { name: "monitoring" } },
  { provider: eksCluster.provider }
);
const monitoringServiceAccount = new k8s.core.v1.ServiceAccount("monitoringServiceAccount", {
  metadata: {
    namespace: monitoringNamespace.metadata.name,
    name: "vector-service-account", // required by betterstack
  },
});
const monitoringServiceAccountSecret = new k8s.core.v1.Secret(
  "monitoringServiceAccountSecret",
  {
    metadata: {
      namespace: monitoringNamespace.metadata.name,
      name: monitoringServiceAccount.metadata.name,
      annotations: {
        "kubernetes.io/service-account.name": monitoringServiceAccount.metadata.name,
      },
    },
    type: "kubernetes.io/service-account-token",
  },
  { provider: eksCluster.provider }
);

const monitoringClusterRole = new k8s.rbac.v1.ClusterRole("monitoringClusterRole", {
  rules: [
    {
      apiGroups: ["*"],
      resources: ["*"],
      verbs: ["get", "list", "watch"],
    },
  ],
});
const monitoringClusterRoleBinding = new k8s.rbac.v1.ClusterRoleBinding("monitoringClusterRoleBinding", {
  subjects: [
    {
      kind: "ServiceAccount",
      name: monitoringServiceAccount.metadata.name,
      namespace: monitoringServiceAccount.metadata.namespace,
    },
  ],
  roleRef: {
    kind: "ClusterRole",
    name: monitoringClusterRole.metadata.name,
    apiGroup: "rbac.authorization.k8s.io",
  },
});

const betterstackToken = config.requireSecret("BETTERSTACK_SECRET");
const betterstack = new k8s.helm.v3.Chart(
  "betterstack-logs",
  {
    namespace: monitoringNamespace.metadata.name,
    chart: "betterstack-logs",
    fetchOpts: {
      repo: "https://betterstackhq.github.io/logs-helm-chart",
    },
    values: {
      vector: {
        customConfig: {
          sinks: {
            better_stack_http_sink: {
              auth: {
                token: betterstackToken,
              },
            },
            better_stack_http_metrics_sink: {
              auth: {
                token: betterstackToken,
              },
            },
          },
          // stolen from generated config map to accept insecure TLS
          sources: {
            better_stack_kubernetes_logs: {
              type: "kubernetes_logs",
            },
            better_stack_kubernetes_metrics_nodes: {
              auth: {
                strategy: "bearer",
                token: "$SERVICE_ACCOUNT_TOKEN",
              },
              decoding: {
                codec: "json",
              },
              endpoint: "https://betterstack-logs-metrics-server/apis/metrics.k8s.io/v1beta1/nodes",
              headers: {
                accept: ["application/json"],
              },
              tls: {
                verify_certificate: false,
              },
              type: "http_client",
            },
            better_stack_kubernetes_metrics_pods: {
              auth: {
                strategy: "bearer",
                token: "$SERVICE_ACCOUNT_TOKEN",
              },
              decoding: {
                codec: "json",
              },
              endpoint: "https://betterstack-logs-metrics-server/apis/metrics.k8s.io/v1beta1/pods",
              headers: {
                accept: ["application/json"],
              },
              tls: {
                verify_certificate: false,
              },
              type: "http_client",
            },
          },
        },
        serviceAccount: {
          create: false,
          name: monitoringServiceAccount.metadata.name,
          automountToken: true,
        },
      },
    },
  },
  {
    provider: eksCluster.provider,
    dependsOn: [monitoringNamespace, monitoringServiceAccount, monitoringServiceAccountSecret],
  }
);
