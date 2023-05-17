import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as eks from "@pulumi/eks";
import * as k8s from "@pulumi/kubernetes";
import * as pulumi from "@pulumi/pulumi";
import { getBenchUserS3AccessKey, makeALBController, makeEbsCsiDriver } from "./aws";

// configuration
const config = new pulumi.Config();
const minClusterSize = config.getNumber("minClusterSize") || 2;
const maxClusterSize = config.getNumber("maxClusterSize") || 3;
const desiredClusterSize = config.getNumber("desiredClusterSize") || 2;
const eksNodeInstanceType = config.get("eksNodeInstanceType") || "t3.medium";
const vpcNetworkCidr = config.get("vpcNetworkCidr") || "10.0.0.0/16";

// Create a new VPC
const eksVpc = new awsx.ec2.Vpc("eks-vpc", {
  enableDnsHostnames: true,
  cidrBlock: vpcNetworkCidr,
  subnetSpecs: [
    { type: "Public", tags: { "kubernetes.io/role/elb": "1" } },
    { type: "Private", tags: { "kubernetes.io/role/internal-elb": "1" } },
  ],
});

// Create the EKS cluster
const eksCluster = new eks.Cluster("eks-cluster", {
  name: "bench-" + config.require("env"),
  vpcId: eksVpc.vpcId,
  // Public subnets will be used for load balancers
  publicSubnetIds: eksVpc.publicSubnetIds,
  // Private subnets will be used for cluster nodes
  privateSubnetIds: eksVpc.privateSubnetIds,
  instanceType: eksNodeInstanceType,
  desiredCapacity: desiredClusterSize,
  minSize: minClusterSize,
  maxSize: maxClusterSize,
  nodeAssociatePublicIpAddress: false,
});
// add annotations for ALB to subnet tags

// Image pull secrets for GHCR
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

// Create AWS ALB Ingress Controller
const albIngressController = makeALBController(eksVpc, eksCluster);

// Create AWS EBS CSI Driver
const ebsCsiDriver = makeEbsCsiDriver(eksVpc, eksCluster);

//
// Core application
//

// Create RDS Aurora Postgres cluster/database
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
const db = new aws.rds.Cluster("db", {
  engine: "aurora-postgresql",
  clusterIdentifier: "db",
  engineVersion: "14.3",
  databaseName: "postgres",
  deletionProtection: true,
  masterUsername: "postgres",
  masterPassword: config.requireSecret("dbPassword"),
  backupRetentionPeriod: 7,
  preferredBackupWindow: "04:00-06:00",
  vpcSecurityGroupIds: [dbSecurityGroup.id],
});
const dbInstance = new aws.rds.ClusterInstance("db", {
  clusterIdentifier: db.clusterIdentifier,
  instanceClass: "db.t3.medium",
  engine: "aurora-postgresql",
  engineVersion: "14.3",
  publiclyAccessible: true,
  performanceInsightsEnabled: true,
});
const dbSecret = new k8s.core.v1.Secret(
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
const DB_ENV_VARS = [
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
        name: dbSecret.metadata.name,
        key: "password",
      },
    },
  },
  {
    name: "BENCH_POSTGRES_HOST",
    value: db.endpoint,
  },
  {
    name: "BENCH_POSTGRES_PORT",
    value: db.port.apply((port) => port.toString()),
  },
  {
    name: "PGCRYPTO_KEY",
    value: config.requireSecret("pgcryptoKey"),
  },
];

// Create persistent ElastiCache Redis cluster
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
const redis = new aws.elasticache.Cluster("redis", {
  engine: "redis",
  nodeType: "cache.t3.micro",
  numCacheNodes: 1,
  port: 6379,
  parameterGroupName: "default.redis7",
  securityGroupIds: [redisSecurityGroup.id],
  subnetGroupName: redisSubnetGroup.id,
});

// NATS chart
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
    accessKeyId: s3AccessKey.accessKeyId.apply((id) => Buffer.from(id).toString("base64")),
    secretAccessKey: s3AccessKey.secretAccessKey.apply((secret) => Buffer.from(secret).toString("base64")),
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

// General backend env vars
const BACKEND_ENV_VARS = [
  // sentry
  {
    name: "SENTRY_DSN",
    value: config.requireSecret("SENTRY_DSN"),
  },
  // nats
  {
    name: "NATS_SERVER",
    value: nats.name.apply((name) => `nats://${name}:4222`),
  },
  // redis
  {
    name: "REDIS_URL",
    value: redis.cacheNodes[0].address.apply((address) => `redis://${address}:6379`),
  },
];

// SandboxedWorker env vars
const SECRET_MODEL_PROVIDER_VARS = [
  // provider secrets
  "OPENAI_API_KEY",
  "GOOSEAI_API_KEY",
  "FOREFRONT_API_KEY",
  "COHERE_API_KEY",
  "ANTHROPIC_API_KEY",
].map((name) => ({
  name,
  value: config.requireSecret(name),
}));

// Public load-balanced API service (also runs internal server)
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

// Internal worker service
const workerName = "worker";

// Use git commit hashes as version by default
const version = config.require("version");
// if version is 'current', get the current commit hash
let imageVersion;
if (version == "current") {
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  imageVersion = require("child_process").execSync("git rev-parse --short HEAD").toString().trim();
} else {
  imageVersion = version;
}

const SOCIAL_AUTH_ENV_VARS = [
  "SOCIAL_AUTH_GITHUB_KEY",
  "SOCIAL_AUTH_GITHUB_SECRET",
  "SOCIAL_AUTH_GITLAB_KEY",
  "SOCIAL_AUTH_GITLAB_SECRET",
  "SOCIAL_AUTH_GOOGLE_OAUTH2_KEY",
  "SOCIAL_AUTH_GOOGLE_OAUTH2_SECRET",
].map((name) => ({
  name,
  value: config.requireSecret(name),
}));

// Create deployment for API service (ASGI Django with Daphne)
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
          // auto-migrate
          initContainers: [
            {
              name: apiName + "-migrate",
              image: `ghcr.io/symbolx/bench-api:${imageVersion}`,
              env: [...BACKEND_ENV_VARS, ...DB_ENV_VARS, { name: "SEND_API_PUB_MSG", value: "" }],
              command: ["python", "manage.py", "migrate"],
            },
          ],
          // launch daphne
          containers: [
            {
              name: apiName,
              image: `ghcr.io/symbolx/bench-api:${imageVersion}`,
              ports: [{ containerPort: 80, name: "http" }],
              env: [
                ...BACKEND_ENV_VARS,
                ...DB_ENV_VARS,
                ...SECRET_MODEL_PROVIDER_VARS, // needed for running langserver
                ...AWS_BACKEND_ENV_VARS,
                { name: "ALLOWED_HOSTS", value: config.require("apiAllowedHosts") },
                { name: "CORS_ALLOWED_ORIGINS", value: config.require("apiAllowedOrigins") },
                { name: "WEBAPP_URL", value: config.require("webappUrl") },
                { name: "RUN_INTSERVER", value: "true" },
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
// Create deployment for workers
const workerDeployment = new k8s.apps.v1.Deployment(
  workerName,
  {
    metadata: { namespace: "default", labels: { app: workerName } },
    spec: {
      replicas: 1,
      selector: { matchLabels: { app: workerName } },
      template: {
        metadata: { labels: { app: workerName }, annotations: { "prometheus.io/scrape": "true" } },
        spec: {
          containers: [
            {
              name: workerName,
              image: `ghcr.io/symbolx/bench-api:${imageVersion}`,
              ports: [{ containerPort: 80, name: "http" }],
              env: [
                ...BACKEND_ENV_VARS,
                ...SECRET_MODEL_PROVIDER_VARS,
                { name: "ALLOW_UNTRUSTED_CODE", value: "true" },
              ],
              command: ["python", "bench/runworker.py"],
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

// Expose API service via HTTPS ingress
const apiDomain = "api.symbolx.com";
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
        "certificate-arn": "arn:aws:acm:eu-central-1:163349077661:certificate/ca07536d-3af0-419c-a29b-d27237cd4a6d",
      },
      namespace: "default",
    },
    spec: {
      ingressClassName: "alb",
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

//
// Monitoring
//

const monitoringNs = new k8s.core.v1.Namespace("monitoring", {}, { provider: eksCluster.provider });

const kubecostNs = new k8s.core.v1.Namespace("kubecost", {}, { provider: eksCluster.provider });
const kubecost = new k8s.helm.v3.Release(
  "kubecost",
  {
    chart: "cost-analyzer",
    repositoryOpts: {
      repo: "https://kubecost.github.io/cost-analyzer",
    },
    namespace: kubecostNs.metadata.name,
    version: "1.89.1",
    values: {
      persistentVolume: {
        enabled: true,
        storageClass: "gp2",
      },
      kubecostToken: config.requireSecret("kubecostToken"),
      // include prometheus but not metrics server
      // see https://docs.kubecost.com/install-and-configure/install/custom-prom
      prometheus: {
        nodeExporter: {
          enabled: false,
        },
        serviceAccounts: {
          nodeExporter: {
            create: false,
          },
        },
        kubeStateMetrics: {
          enabled: false,
        },
      },
    },
  },
  { dependsOn: [ebsCsiDriver] }
);

const kubeStateMetrics = new k8s.helm.v3.Release("kube-state-metrics", {
  chart: "kube-state-metrics",
  version: "5.0.0",
  namespace: "kube-system",
  repositoryOpts: {
    repo: "https://prometheus-community.github.io/helm-charts",
  },
});

const metricsServer = new k8s.helm.v3.Release("metrics-server", {
  chart: "metrics-server",
  version: "3.8.4",
  namespace: "kube-system",
  repositoryOpts: {
    repo: "https://kubernetes-sigs.github.io/metrics-server/",
  },
});

// TODO @Monitoring: dashboard & alert with grafana?
