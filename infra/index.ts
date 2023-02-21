import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as eks from "@pulumi/eks";
import * as k8s from "@pulumi/kubernetes";
import * as pulumi from "@pulumi/pulumi";
import * as random from "@pulumi/random";

// Grab some values from the Pulumi configuration (or use default values)
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

// Create RDS Aurora Postgres cluster/database
const db = new aws.rds.Cluster("db", {
  engine: "aurora-postgresql",
  clusterIdentifier: "db",
  engineVersion: "14.3",
  databaseName: "postgres",
  deletionProtection: true,
  masterUsername: "postgres",
  masterPassword: config.requireSecret("dbPassword"),
});
const dbInstance = new aws.rds.ClusterInstance("db", {
  clusterIdentifier: db.clusterIdentifier,
  instanceClass: "db.t3.medium",
  engine: "aurora-postgresql",
  engineVersion: "14.3",
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
const apiService = new k8s.core.v1.Service(
  "api",
  {
    spec: {
      type: "LoadBalancer",
      ports: [
        { port: 80, name: "http" },
        { port: 5555, name: "zmq-1" },
        { port: 5556, name: "zmq-2" },
        { port: 5557, name: "zmq-3" },
      ],
      selector: { app: "api" },
    },
  },
  { provider: eksCluster.provider }
);
// Internal worker service
const workerService = new k8s.core.v1.Service(
  "worker",
  {
    metadata: { namespace: "default" },
    spec: {
      type: "ClusterIP",
      ports: [
        { port: 5558, name: "zmq-1" },
        { port: 5559, name: "zmq-2" },
      ],
      selector: { app: "worker" },
    },
  },
  { provider: eksCluster.provider }
);

// zmq env vars to connect them
const ZMQ_ENV_VARS: { name: string; value: string }[] = [
  // {
  //   name: "ZMQ_API_PUB_ADDR",
  //   value: apiService.status.loadBalancer.ingress[0].hostname.apply((hostname) => `tcp://${hostname}:5555`),
  // },
  // {
  //   name: "ZMQ_INTSERVER_PUB_ADDR",
  //   value: apiService.status.loadBalancer.ingress[0].hostname.apply((hostname) => `tcp://${hostname}:5556`),
  // },
  // {
  //   name: "ZMQ_INTSERVER_REP_ADDR",
  //   value: apiService.status.loadBalancer.ingress[0].hostname.apply((hostname) => `tcp://${hostname}:5557`),
  // },
  // {
  //   name: "ZMQ_WORKER_PUB_ADDR",
  //   value: workerService.spec.clusterIP.apply((ip) => `tcp://${ip}:5558`),
  // },
  // {
  //   name: "ZMQ_WORKER_REP_ADDR",
  //   value: workerService.spec.clusterIP.apply((ip) => `tcp://${ip}:5559`),
  // },
];

const imageVersion = config.get("imageVersion") || "latest";
// Create deployment for API service (ASGI Django with Daphne)
const apiDeployment = new k8s.apps.v1.Deployment(
  "api",
  {
    metadata: { namespace: "default", labels: { app: "api" } },
    spec: {
      replicas: 1,
      selector: { matchLabels: { app: "api" } },
      template: {
        metadata: { labels: { app: "api" }, annotations: { "prometheus.io/scrape": "true" } },
        spec: {
          containers: [
            {
              name: "api",
              image: `ghcr.io/symbolx/bench-api:${imageVersion}`,
              ports: [{ containerPort: 80, name: "http" }],
              env: [...DB_ENV_VARS, ...ZMQ_ENV_VARS, { name: "RUN_INTSERVER", value: "true" }],
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
  "worker",
  {
    metadata: { namespace: "default", labels: { app: "worker" } },
    spec: {
      replicas: 1,
      selector: { matchLabels: { app: "worker" } },
      template: {
        metadata: { labels: { app: "worker" }, annotations: { "prometheus.io/scrape": "true" } },
        spec: {
          containers: [
            {
              name: "worker",
              image: `ghcr.io/symbolx/bench-api:${imageVersion}`,
              ports: [{ containerPort: 80 }],
              env: [...ZMQ_ENV_VARS],
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

// TODO @Incomplete: kubecost
// const kubecost = new k8s.helm.v3.Chart("kubecost", {
//   chart: "cost-analyzer",
//   repo: "kubecost",
//   namespace: "kubecost",
//   version: "1.89.1",
//   values: {
//     persistentVolume: {
//       enabled: true,
//       storageClass: "gp2",
//     },
//   },
//   fetchOpts: {
//     repo: "https://kubecost.github.io/cost-analyzer",
//   },
// });

// TODO @Incomplete: cert-manager for ... certs
// const certManager = new k8s.helm.v3.Chart("cert-manager", {
//   chart: "cert-manager",
//   repo: "jetstack",
//   namespace: "cert-manager",
//   version: "v1.11.0",
//   fetchOpts: {
//     repo: "https://charts.jetstack.io",
//   },
//   values: {
//     installCRDs: true,
//   },
// });

// TODO @Incomplete: metrics server
// const metricsServer = new k8s.helm.v3.Chart("metrics-server", {
//   chart: "metrics-server",
//   repo: "metrics-server",
//   version: "3.8.2",
//   namespace: "kube-system",
//   fetchOpts: {
//     repo: "https://charts.bitnami.com",
//   },
// });

// TODO @Incomplete: prometheus
