import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as eks from "@pulumi/eks";
import * as k8s from "@pulumi/kubernetes";
import * as pulumi from "@pulumi/pulumi";
import { makeALBController } from "./alb";

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
makeALBController(eksVpc, eksCluster);

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
];

// Public load-balanced API service (also runs internal server)
const apiName = "api";
const apiService = new k8s.core.v1.Service(
  apiName,
  {
    spec: {
      type: "NodePort",
      ports: [
        { port: 80, name: "http" },
        { port: 5555, name: "zmq-1" },
        { port: 5556, name: "zmq-2" },
        { port: 5557, name: "zmq-3" },
      ],
      selector: { app: apiName },
    },
  },
  { provider: eksCluster.provider }
);

// Internal worker service
const workerName = "worker";
const workerService = new k8s.core.v1.Service(
  workerName,
  {
    metadata: { namespace: "default" },
    spec: {
      type: "ClusterIP",
      ports: [
        { port: 5558, name: "zmq-1" },
        { port: 5559, name: "zmq-2" },
      ],
      selector: { app: workerName },
    },
  },
  { provider: eksCluster.provider }
);

// zmq env vars to connect them
const ZMQ_ENV_VARS = [
  {
    name: "ZMQ_API_PUB_ADDR",
    value: `tcp://${apiName}:5555`,
  },
  {
    name: "ZMQ_INTSERVER_PUB_ADDR",
    value: `tcp://${apiName}:5556`,
  },
  {
    name: "ZMQ_INTSERVER_REP_ADDR",
    value: `tcp://${apiName}:5557`,
  },
  {
    name: "ZMQ_WORKER_PUB_ADDR",
    value: `tcp://${workerName}:5558`,
  },
  {
    name: "ZMQ_WORKER_REP_ADDR",
    value: `tcp://${workerName}:5559`,
  },
];
// api service should have api addresses set to localhost
const ZMQ_API_ENV_VARS = ZMQ_ENV_VARS.map((envVar) => {
  if (envVar.name.includes("API") || envVar.name.includes("INTSERVER")) {
    return { ...envVar, value: envVar.value.replace("api", "*") };
  }
  return envVar;
});
// worker service should have worker addresses set to localhost
const ZMQ_WORKER_ENV_VARS = ZMQ_ENV_VARS.map((envVar) => {
  if (envVar.name.includes("WORKER")) {
    return { ...envVar, value: envVar.value.replace("worker", "*") };
  }
  return envVar;
});

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
              env: [...DB_ENV_VARS, ...ZMQ_API_ENV_VARS, { name: "SEND_API_PUB_MSG", value: "" }],
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
                ...DB_ENV_VARS,
                ...ZMQ_API_ENV_VARS,
                { name: "ALLOWED_HOSTS", value: config.require("apiAllowedHosts") },
                { name: "RUN_INTSERVER", value: "true" },
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
              ports: [{ containerPort: 80 }],
              env: [...ZMQ_WORKER_ENV_VARS],
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

// Kube cost monitoring
// const kubecostNs = new k8s.core.v1.Namespace("kubecost", {}, { provider: eksCluster.provider });
// const kubecost = new k8s.helm.v3.Release("kubecost", {
//   chart: "cost-analyzer",
//   repositoryOpts: {
//     repo: "https://kubecost.github.io/cost-analyzer",
//   },
//   namespace: kubecostNs.metadata.name,
//   version: "1.89.1",
//   values: {
//     persistentVolume: {
//       enabled: true,
//       storageClass: "gp2",
//     },
//   },
// });

// Expose API service via HTTPS ingress
const apiDomain = "api.symbolx.com";
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

// TODO @Incomplete: Export kube metrics with metrics-server
// const metricsServer = new k8s.helm.v3.Release("metrics-server", {
//   chart: "metrics-server",
//   version: "6.8.2",
//   namespace: "kube-system",
//   repositoryOpts: {
//     repo: "https://charts.bitnami.com/bitnami",
//   },
// });

// TODO @Incomplete: prometheus
// const prometheus = new k8s.helm.v3.Resource("prometheus", {
//   chart: "prometheus",
//   version: "14.6.0",
//   namespace: "monitoring",
//   fetchOpts: {
//     repo: "https://prometheus-community.github.io/helm-charts",
//   },
// });
// TODO @Incomplete: grafana?
