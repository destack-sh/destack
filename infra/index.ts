import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as eks from "@pulumi/eks";
import * as k8s from "@pulumi/kubernetes";
import * as pulumi from "@pulumi/pulumi";

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
  // Put the cluster in the new VPC created earlier
  vpcId: eksVpc.vpcId,
  // Public subnets will be used for load balancers
  publicSubnetIds: eksVpc.publicSubnetIds,
  // Private subnets will be used for cluster nodes
  privateSubnetIds: eksVpc.privateSubnetIds,
  // Change configuration values to change any of the following settings
  instanceType: eksNodeInstanceType,
  desiredCapacity: desiredClusterSize,
  minSize: minClusterSize,
  maxSize: maxClusterSize,
  // Do not give the worker nodes public IP addresses
  nodeAssociatePublicIpAddress: false,
  // Uncomment the next two lines for a private cluster (VPN access required)
  // endpointPrivateAccess: true,
  // endpointPublicAccess: false
});

// Create RDS Aurora Postgres database
const db = new aws.rds.Instance("db", {
  engine: "aurora-postgresql",
  allocatedStorage: 20,
  maxAllocatedStorage: 500,
  engineVersion: "14.3",
  instanceClass: "db.t3.micro",
  name: "postgres",
  username: "postgres",
  password: config.requireSecret("dbPassword"),
});

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
        name: "db",
        key: "password",
      },
    },
  },
  {
    name: "BENCH_POSTGRES_HOST",
    value: db.address,
  },
  {
    name: "BENCH_POSTGRES_PORT",
    value: db.port.apply((port) => port.toString()),
  },
];

// zmq env vars
const ZMQ_ENV_VARS = [];

// Create deployment for API server (ASGI Django with Daphne)
const apiDeployment = new k8s.apps.v1.Deployment("api", {
  spec: {
    replicas: 1,
    selector: { matchLabels: { app: "api" } },
    template: {
      metadata: { labels: { app: "api" } },
      spec: {
        containers: [
          {
            name: "api",
            image: "ghcr.io/symbolx/bench-api:latest",
            ports: [{ containerPort: 80 }],
            env: [...DB_ENV_VARS, { name: "RUN_INTSERVER", value: "true" }],
          },
        ],
      },
    },
  },
});

// TODO @Incomplete: create deployment for workers (same image for now)

// Load balance and expose the API server
const apiService = new k8s.core.v1.Service("api", {
  spec: {
    type: "LoadBalancer",
    ports: [{ port: 80, targetPort: 80 }],
    selector: apiDeployment.spec.template.metadata.labels,
  },
});

// TODO @Incomplete: use cert-manager helm chart to create a certificate for the API server

// Export some values for use elsewhere
export const kubeconfig = eksCluster.kubeconfig;
export const vpcId = eksVpc.vpcId;
