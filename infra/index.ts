import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as eks from "@pulumi/eks";
import * as k8s from "@pulumi/kubernetes";
import * as pulumi from "@pulumi/pulumi";
import {
  getBenchUserS3AccessKey,
  makeALBController,
  makeEbsCsiDriver,
  makeOpensearch,
  makeRds,
  secretFrom as secretFrom,
} from "./aws";
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

// DB: RDS Aurora Postgres cluster/database (global and user)
const { dbInstance: globalDbInstance } = makeRds("global-db", "db.t3.medium", {
  password: config.requireSecret("globalDbPassword"),
});
const globalDbSecret = secretFrom("global-db", config.requireSecret("globalDbPassword"), {
  key: "password",
  provider: eksCluster.provider,
});
const GLOBAL_PG_VARS = [
  { name: "GLOBAL_PG_HOST", value: globalDbInstance.endpoint },
  { name: "GLOBAL_PG_NAME", value: "postgres" },
  { name: "GLOBAL_PG_USERNAME", value: "postgres" },
  {
    name: "GLOBAL_PG_PASSWORD",
    valueFrom: {
      secretKeyRef: {
        name: globalDbSecret.metadata.name,
        key: "password",
      },
    },
  },
  { name: "GLOBAL_PG_PORT", value: globalDbInstance.port.apply((port) => port.toString()) },
  { name: "GLOBAL_PG_CRYPTO_KEY", value: config.requireSecret("globalPgCryptoKey") },
];
const { dbInstance: userDbInstance } = makeRds("user-db", "db.t3.medium", {
  password: config.requireSecret("userDbPassword"),
});
const userDbSecret = secretFrom("user-db", config.requireSecret("userDbPassword"), {
  key: "password",
  provider: eksCluster.provider,
});
const USER_PG_VARS = [
  { name: "USER_PG_HOST", value: userDbInstance.endpoint },
  { name: "USER_PG_NAME", value: "postgres" },
  { name: "USER_PG_USERNAME", value: "postgres" },
  {
    name: "USER_PG_PASSWORD",
    valueFrom: {
      secretKeyRef: {
        name: userDbSecret.metadata.name,
        key: "password",
      },
    },
  },
  { name: "USER_PG_PORT", value: userDbInstance.port.apply((port) => port.toString()) },
];

// Search: OpenSearch cluster (shared between global and user for now)
const { osDomain: userOsDomain } = makeOpensearch("user-os", 1, "t3.medium.search", {
  password: config.requireSecret("userOsPassword"),
  vpc: eksVpc,
  region: config.require("awsRegion"),
});
const userOsSecret = secretFrom("user-os", config.requireSecret("userOsPassword"), {
  key: "password",
  provider: eksCluster.provider,
});
const USER_OS_VARS = [
  { name: "USER_OS_HOST", value: userOsDomain.endpoint },
  { name: "USER_OS_PORT", value: "443" },
  { name: "USER_OS_USERNAME", value: "opensearch" },
  {
    name: "USER_OS_PASSWORD",
    valueFrom: {
      secretKeyRef: {
        name: userOsSecret.metadata.name,
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
  // TODO :Security!: don't give worker user full Redis access
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
const AWS_BACKEND_VARS = [
  { name: "AWS_REGION", value: config.require("awsRegion") },
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
    name: "JSON_LOGS",
    value: "1",
  },
  {
    name: "ENVIRONMENT",
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
];

const PRIVATE_BACKEND_VARS = [
  "OPENAI_API_KEY",
  "ANTHROPIC_API_KEY",
  "DEEPGRAM_API_KEY",
  "HUGGINGFACE_API_KEY",
  "BROWSERLESS_API_KEY",
].map((name) => ({
  name,
  value: config.requireSecret(name),
}));


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

const BASE_PRIVATE_BACKEND_VARS = [
  { name: "LOOPS_API_KEY", value: config.requireSecret("LOOPS_API_KEY") },
  { name: "LOOPS_USER_TRANSACTIONAL_ID", value: config.require("LOOPS_USER_TRANSACTIONAL_ID") },
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
const KUBERNETES_VARS = [
  { name: "KUBERNETES_WORKER_IMAGE", value: `ghcr.io/symbolx/bench-worker:${imageVersion}` },
  { name: "KUBERNETES_WORKER_ENV_VARS", value: WORKER_ENV_VARS_ENCODED },
  { name: "KUBERNETES_WORKER_IMAGE_PULL_SECRET_NAME", value: imagePullSecret.metadata.name },
];

// get envoy.yaml from this folder and put into configmap
// (also replace :EnvoyLocalhost with actual localhost)
const envoyConfigString = fs.readFileSync("envoy.yaml", "utf8")
const envoyConfig = yaml.load(envoyConfigString.replace('host.docker.internal', 'localhost'));
const envoyConfigMap = new k8s.core.v1.ConfigMap(
  "envoy-config",
  {
    metadata: { namespace: "default" },
    data: { "envoy.yaml": yaml.dump(envoyConfig) },
  },
  { provider: eksCluster.provider }
);

// master server (unsharded / 1 instance for now)
const serverName = "server";
const serverService = new k8s.core.v1.Service(
  serverName,
  {
    spec: {
      type: "NodePort",
      ports: [
        { port: 80, name: "http" },
        { port: 8080, name: "grpc-web" },
      ],
      selector: { app: serverName },
    },
  },
  { provider: eksCluster.provider, protect: true }
);
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
          initContainers: [
            // auto-migrate
            {
              name: serverName + "-migrate",
              image: `ghcr.io/symbolx/bench-system:${imageVersion}`,
              env: [
                ...PUBLIC_BACKEND_VARS,
                ...GLOBAL_PG_VARS,
                ...USER_PG_VARS,
                ...USER_OS_VARS,
                ...AWS_BACKEND_VARS,
                ...BASE_PRIVATE_BACKEND_VARS,
              ],
              command: ["/bin/sh", "-c"],
              // TODO :Robustness!: probably don't want to migrate the local Bench DB's all at once
              args: ["python bench.py sql migrate && python bench.py sql migrate --bench '*'"],
            },
          ],
          containers: [
            // envoy sidecar to proxy http -> grpc
            {
              name: "envoy",
              image: "envoyproxy/envoy:v1.28-latest",
              ports: [{ containerPort: 8080, name: "grpc-web" }],
              volumeMounts: [
                {
                  name: "envoy-config",
                  mountPath: "/etc/envoy",
                  readOnly: true,
                },
              ],
            },
            // main server
            {
              name: serverName,
              image: `ghcr.io/symbolx/bench-system:${imageVersion}`,
              ports: [
                { containerPort: 80, name: "http" },
                { containerPort: 50051, name: "grpc" },
              ],
              env: [
                ...PUBLIC_BACKEND_VARS,
                ...GLOBAL_PG_VARS,
                ...USER_PG_VARS,
                ...USER_OS_VARS,
                ...PRIVATE_BACKEND_VARS,
                ...AWS_BACKEND_VARS,
                ...BASE_PRIVATE_BACKEND_VARS,
                ...KUBERNETES_VARS,
              ],
              command: ["python", "manageserver.py", "all"],
              resources: { requests: { cpu: "2000m", memory: "2000Mi" } },
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
          volumes: [
            {
              name: "envoy-config",
              configMap: { name: envoyConfigMap.metadata.name },
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

// Expose server via HTTPS ingress
const serverDomain = "server.justbench.com";
// TODO @Infra: manage AWS certificate via aws.acm.Certificate
// (without causing issues with current certificate)
const serverIngress = new k8s.networking.v1.Ingress(
  serverName,
  {
    metadata: {
      annotations: {
        "kubernetes.io/ingress.class": "alb",
        "alb.ingress.kubernetes.io/ssl-redirect": "443",
        "alb.ingress.kubernetes.io/listen-ports": '[{"HTTP": 80}, {"HTTPS":443}]',
        "alb.ingress.kubernetes.io/scheme": "internet-facing",
        "alb.ingress.kubernetes.io/target-type": "ip",
        // stickiness for our grpc-web connections
        "alb.ingress.kubernetes.io/target-group-attributes":
          "stickiness.enabled=true,stickiness.type=lb_cookie,stickiness.lb_cookie.duration_seconds=86400",
        // ALB doesn't support cert-manager certs, so we need to provision that cert ACM
        "certificate-arn": "arn:aws:acm:eu-central-1:163349077661:certificate/8271c03a-0830-4c02-81af-b5add9429291",
      },
      namespace: "default",
    },
    spec: {
      tls: [{ hosts: [serverDomain], secretName: "server-cert" }],
      rules: [
        {
          host: serverDomain,
          http: {
            paths: [
              {
                path: "/",
                pathType: "Prefix",
                backend: {
                  service: {
                    name: serverService.metadata.name,
                    port: { number: 8080 },
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
// TODO @Infra :Broken: setting up monitoring with the proper service account fails
//  which means that kubernetes metrics aren't properly reported
//  It fails because both we and the chart try to create the service account secret
//  (even though the chart shouldn't, we disable its service account to give it our own...)
// const monitoringServiceAccount = new k8s.core.v1.ServiceAccount(
//   "monitoringServiceAccount",
//   {
//     metadata: {
//       namespace: monitoringNamespace.metadata.name,
//       name: "vector-service-account", // required by betterstack / Vector
//     },
//   },
//   { aliases: [{ name: "monitoringServiceAccount" }] }
// );
// const monitoringServiceAccountSecret = new k8s.core.v1.Secret(
//   "monitoringServiceAccountSecret",
//   {
//     metadata: {
//       namespace: monitoringNamespace.metadata.name,
//       name: monitoringServiceAccount.metadata.name,
//       annotations: {
//         "kubernetes.io/service-account.name": monitoringServiceAccount.metadata.name,
//       },
//     },
//     type: "kubernetes.io/service-account-token",
//   },
//   { provider: eksCluster.provider }
// );

// const monitoringClusterRole = new k8s.rbac.v1.ClusterRole("monitoringClusterRole", {
//   rules: [
//     {
//       apiGroups: ["*"],
//       resources: ["*"],
//       verbs: ["get", "list", "watch"],
//     },
//   ],
// });
// const monitoringClusterRoleBinding = new k8s.rbac.v1.ClusterRoleBinding("monitoringClusterRoleBinding", {
//   subjects: [
//     {
//       kind: "ServiceAccount",
//       name: monitoringServiceAccount.metadata.name,
//       namespace: monitoringServiceAccount.metadata.namespace,
//     },
//   ],
//   roleRef: {
//     kind: "ClusterRole",
//     name: monitoringClusterRole.metadata.name,
//     apiGroup: "rbac.authorization.k8s.io",
//   },
// });

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
        // serviceAccount: {
        //   create: false,
        //   name: monitoringServiceAccount.metadata.name,
        //   automountToken: true,
        // },
      },
    },
  },
  {
    provider: eksCluster.provider,
    // dependsOn: [monitoringNamespace, monitoringServiceAccount, monitoringServiceAccountSecret],
  }
);
