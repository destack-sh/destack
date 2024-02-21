import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as eks from "@pulumi/eks";
import * as k8s from "@pulumi/kubernetes";
import { Output, output } from "@pulumi/pulumi";

// see https://www.pulumi.com/blog/kubernetes-ingress-with-aws-alb-ingress-controller-and-pulumi-crosswalk/

export function makeALBController(vpc: awsx.ec2.Vpc, cluster: eks.Cluster) {
  // setup AWS ALB ingress controller
  // Create IAM Policy for the IngressController called "ingressController-iam-policy” and read the policy ARN.
  const ingressControllerPolicy = new aws.iam.Policy("ingressController-iam-policy", {
    policy: {
      Version: "2012-10-17",
      Statement: [
        {
          Effect: "Allow",
          Action: [
            "iam:CreateServiceLinkedRole",
            "ec2:DescribeAccountAttributes",
            "ec2:DescribeAddresses",
            "ec2:DescribeAvailabilityZones",
            "ec2:DescribeInternetGateways",
            "ec2:DescribeVpcs",
            "ec2:DescribeSubnets",
            "ec2:DescribeSecurityGroups",
            "ec2:DescribeInstances",
            "ec2:DescribeNetworkInterfaces",
            "ec2:DescribeTags",
            "ec2:GetCoipPoolUsage",
            "ec2:DescribeCoipPools",
            "elasticloadbalancing:DescribeLoadBalancers",
            "elasticloadbalancing:DescribeLoadBalancerAttributes",
            "elasticloadbalancing:DescribeListeners",
            "elasticloadbalancing:DescribeListenerCertificates",
            "elasticloadbalancing:DescribeSSLPolicies",
            "elasticloadbalancing:DescribeRules",
            "elasticloadbalancing:DescribeTargetGroups",
            "elasticloadbalancing:DescribeTargetGroupAttributes",
            "elasticloadbalancing:DescribeTargetHealth",
            "elasticloadbalancing:DescribeTags",
          ],
          Resource: "*",
        },
        {
          Effect: "Allow",
          Action: [
            "cognito-idp:DescribeUserPoolClient",
            "acm:ListCertificates",
            "acm:DescribeCertificate",
            "iam:ListServerCertificates",
            "iam:GetServerCertificate",
            "waf-regional:GetWebACL",
            "waf-regional:GetWebACLForResource",
            "waf-regional:AssociateWebACL",
            "waf-regional:DisassociateWebACL",
            "wafv2:GetWebACL",
            "wafv2:GetWebACLForResource",
            "wafv2:AssociateWebACL",
            "wafv2:DisassociateWebACL",
            "shield:GetSubscriptionState",
            "shield:DescribeProtection",
            "shield:CreateProtection",
            "shield:DeleteProtection",
          ],
          Resource: "*",
        },
        {
          Effect: "Allow",
          Action: ["ec2:AuthorizeSecurityGroupIngress", "ec2:RevokeSecurityGroupIngress"],
          Resource: "*",
        },
        {
          Effect: "Allow",
          Action: ["ec2:CreateSecurityGroup"],
          Resource: "*",
        },
        {
          Effect: "Allow",
          Action: ["ec2:CreateTags"],
          Resource: "arn:aws:ec2:*:*:security-group/*",
          Condition: {
            StringEquals: {
              "ec2:CreateAction": "CreateSecurityGroup",
            },
            Null: {
              "aws:RequestTag/elbv2.k8s.aws/cluster": "false",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["ec2:CreateTags", "ec2:DeleteTags"],
          Resource: "arn:aws:ec2:*:*:security-group/*",
          Condition: {
            Null: {
              "aws:RequestTag/elbv2.k8s.aws/cluster": "true",
              "aws:ResourceTag/elbv2.k8s.aws/cluster": "false",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["ec2:AuthorizeSecurityGroupIngress", "ec2:RevokeSecurityGroupIngress", "ec2:DeleteSecurityGroup"],
          Resource: "*",
          Condition: {
            Null: {
              "aws:ResourceTag/elbv2.k8s.aws/cluster": "false",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["elasticloadbalancing:CreateLoadBalancer", "elasticloadbalancing:CreateTargetGroup"],
          Resource: "*",
          Condition: {
            Null: {
              "aws:RequestTag/elbv2.k8s.aws/cluster": "false",
            },
          },
        },
        {
          Effect: "Allow",
          Action: [
            "elasticloadbalancing:CreateListener",
            "elasticloadbalancing:DeleteListener",
            "elasticloadbalancing:CreateRule",
            "elasticloadbalancing:DeleteRule",
          ],
          Resource: "*",
        },
        {
          Effect: "Allow",
          Action: ["elasticloadbalancing:AddTags", "elasticloadbalancing:RemoveTags"],
          Resource: [
            "arn:aws:elasticloadbalancing:*:*:targetgroup/*/*",
            "arn:aws:elasticloadbalancing:*:*:loadbalancer/net/*/*",
            "arn:aws:elasticloadbalancing:*:*:loadbalancer/app/*/*",
          ],
          Condition: {
            Null: {
              "aws:RequestTag/elbv2.k8s.aws/cluster": "true",
              "aws:ResourceTag/elbv2.k8s.aws/cluster": "false",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["elasticloadbalancing:AddTags", "elasticloadbalancing:RemoveTags"],
          Resource: [
            "arn:aws:elasticloadbalancing:*:*:listener/net/*/*/*",
            "arn:aws:elasticloadbalancing:*:*:listener/app/*/*/*",
            "arn:aws:elasticloadbalancing:*:*:listener-rule/net/*/*/*",
            "arn:aws:elasticloadbalancing:*:*:listener-rule/app/*/*/*",
          ],
        },
        {
          Effect: "Allow",
          Action: [
            "elasticloadbalancing:ModifyLoadBalancerAttributes",
            "elasticloadbalancing:SetIpAddressType",
            "elasticloadbalancing:SetSecurityGroups",
            "elasticloadbalancing:SetSubnets",
            "elasticloadbalancing:DeleteLoadBalancer",
            "elasticloadbalancing:ModifyTargetGroup",
            "elasticloadbalancing:ModifyTargetGroupAttributes",
            "elasticloadbalancing:DeleteTargetGroup",
          ],
          Resource: "*",
          Condition: {
            Null: {
              "aws:ResourceTag/elbv2.k8s.aws/cluster": "false",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["elasticloadbalancing:RegisterTargets", "elasticloadbalancing:DeregisterTargets"],
          Resource: "arn:aws:elasticloadbalancing:*:*:targetgroup/*/*",
        },
        {
          Effect: "Allow",
          Action: [
            "elasticloadbalancing:SetWebAcl",
            "elasticloadbalancing:ModifyListener",
            "elasticloadbalancing:AddListenerCertificates",
            "elasticloadbalancing:RemoveListenerCertificates",
            "elasticloadbalancing:ModifyRule",
          ],
          Resource: "*",
        },
      ],
    },
  });

  // Attach this policy to the NodeInstanceRole of the worker nodes.
  cluster.instanceRoles.apply((roles) => {
    roles.forEach((role) => {
      new aws.iam.RolePolicyAttachment(
        "eks-NodeInstanceRole-policy-attach-ingressControllerPolicy",
        {
          policyArn: ingressControllerPolicy.arn,
          role: role.name,
        },
        {
          aliases: [{ name: "eks-NodeInstanceRole-policy-attach" }],
        }
      );
    });
  });

  // Declare the ALBIngressController in 1 step with the Helm Chart.
  const albIngressController = new k8s.helm.v3.Release("alb", {
    chart: "aws-load-balancer-controller",
    version: "1.4.7",
    namespace: "kube-system",
    repositoryOpts: {
      repo: "https://aws.github.io/eks-charts",
    },
    values: {
      clusterName: cluster.eksCluster.name,
      region: aws.config.region,
      vpcId: vpc.vpcId,
      serviceAccount: {
        create: true,
        name: "aws-load-balancer-controller",
      },
    },
  });
  return albIngressController;
}

export function makeEbsCsiDriver(vpc: awsx.ec2.Vpc, cluster: eks.Cluster) {
  // Create AWS IAM Role for EBS CSI Driver
  // from https://github.com/kubernetes-sigs/aws-ebs-csi-driver/blob/master/docs/example-iam-policy.json
  const ebsCsiDriverPolicy = new aws.iam.Policy("ebs-csi-driver-policy", {
    policy: {
      Version: "2012-10-17",
      Statement: [
        {
          Effect: "Allow",
          Action: [
            "ec2:CreateSnapshot",
            "ec2:AttachVolume",
            "ec2:DetachVolume",
            "ec2:ModifyVolume",
            "ec2:DescribeAvailabilityZones",
            "ec2:DescribeInstances",
            "ec2:DescribeSnapshots",
            "ec2:DescribeTags",
            "ec2:DescribeVolumes",
            "ec2:DescribeVolumesModifications",
          ],
          Resource: "*",
        },
        {
          Effect: "Allow",
          Action: ["ec2:CreateTags"],
          Resource: ["arn:aws:ec2:*:*:volume/*", "arn:aws:ec2:*:*:snapshot/*"],
          Condition: {
            StringEquals: {
              "ec2:CreateAction": ["CreateVolume", "CreateSnapshot"],
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["ec2:DeleteTags"],
          Resource: ["arn:aws:ec2:*:*:volume/*", "arn:aws:ec2:*:*:snapshot/*"],
        },
        {
          Effect: "Allow",
          Action: ["ec2:CreateVolume"],
          Resource: "*",
          Condition: {
            StringLike: {
              "aws:RequestTag/ebs.csi.aws.com/cluster": "true",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["ec2:CreateVolume"],
          Resource: "*",
          Condition: {
            StringLike: {
              "aws:RequestTag/CSIVolumeName": "*",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["ec2:DeleteVolume"],
          Resource: "*",
          Condition: {
            StringLike: {
              "ec2:ResourceTag/ebs.csi.aws.com/cluster": "true",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["ec2:DeleteVolume"],
          Resource: "*",
          Condition: {
            StringLike: {
              "ec2:ResourceTag/CSIVolumeName": "*",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["ec2:DeleteVolume"],
          Resource: "*",
          Condition: {
            StringLike: {
              "ec2:ResourceTag/kubernetes.io/created-for/pvc/name": "*",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["ec2:DeleteSnapshot"],
          Resource: "*",
          Condition: {
            StringLike: {
              "ec2:ResourceTag/CSIVolumeSnapshotName": "*",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["ec2:DeleteSnapshot"],
          Resource: "*",
          Condition: {
            StringLike: {
              "ec2:ResourceTag/ebs.csi.aws.com/cluster": "true",
            },
          },
        },
        {
          Effect: "Allow",
          Action: ["kms:Decrypt", "kms:GenerateDataKeyWithoutPlaintext", "kms:CreateGrant"],
          Resource: "*",
        },
      ],
    },
  });

  // Attach this policy to the NodeInstanceRole of the worker nodes.
  cluster.instanceRoles.apply((roles) => {
    roles.forEach((role) => {
      new aws.iam.RolePolicyAttachment("eks-NodeInstanceRole-policy-attach-ebsCsiDriverPolicy", {
        policyArn: ebsCsiDriverPolicy.arn,
        role: role.name,
      });
    });
  });

  // Declare the EBS CSI Driver in 1 step with the Helm Chart.
  const ebsCsiDriver = new k8s.helm.v3.Release("ebs-csi-driver", {
    chart: "aws-ebs-csi-driver",
    version: "2.13.0",
    namespace: "kube-system",
    repositoryOpts: {
      repo: "https://kubernetes-sigs.github.io/aws-ebs-csi-driver",
    },
    values: {
      clusterName: cluster.eksCluster.name,
      region: aws.config.region,
      vpcId: vpc.vpcId,
      serviceAccount: {
        create: true,
        name: "aws-ebs-csi-driver",
      },
    },
  });

  return ebsCsiDriver;
}

export function getBenchUserS3AccessKey() {
  // Create a new IAM user
  const s3User = new aws.iam.User("s3User", {});

  // Create the IAM policy for S3 access with a specific prefix
  const s3Policy = new aws.iam.Policy("s3Policy", {
    policy: {
      Version: "2012-10-17",
      Statement: [
        {
          Action: ["s3:ListAllMyBuckets", "s3:GetBucketLocation"],
          Effect: "Allow",
          Resource: "arn:aws:s3:::*",
        },
        {
          Action: "s3:*",
          Effect: "Allow",
          Resource: ["arn:aws:s3:::bench-user-*", "arn:aws:s3:::bench-user-*/*"],
        },
      ],
    },
  });

  // Attach the policy to the user
  const s3UserPolicyAttachment = new aws.iam.UserPolicyAttachment("s3UserPolicyAttachment", {
    user: s3User.name,
    policyArn: s3Policy.arn,
  });

  // Create an access key for the user
  const s3UserAccessKey = new aws.iam.AccessKey("s3UserAccessKey", {
    user: s3User.name,
  });

  return output({
    accessKeyId: s3UserAccessKey.id,
    secretAccessKey: s3UserAccessKey.secret,
  });
}

export function makeRds(name: string, instanceClass: string, config: { password: any; aliases?: string[] }) {
  const extra = { aliases: config.aliases?.map((name) => ({ name })) };
  const dbSecurityGroup = new aws.ec2.SecurityGroup(
    name,
    {
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
    },
    { ...extra }
  );
  const dbCluster = new aws.rds.Cluster(
    name,
    {
      engine: "aurora-postgresql",
      clusterIdentifier: name,
      engineVersion: "14.8",
      databaseName: "postgres",
      deletionProtection: true,
      masterUsername: "postgres",
      masterPassword: config.password,
      backupRetentionPeriod: 7,
      preferredBackupWindow: "04:00-06:00",
      vpcSecurityGroupIds: [dbSecurityGroup.id],
      enabledCloudwatchLogsExports: ["postgresql"],
      storageEncrypted: false, // TODO :Security: make rds storage encrypted
    },
    { ...extra, protect: true }
  );
  const dbInstance = new aws.rds.ClusterInstance(
    name,
    {
      clusterIdentifier: dbCluster.clusterIdentifier,
      instanceClass,
      engine: "aurora-postgresql",
      engineVersion: "14.8",
      publiclyAccessible: true,
      performanceInsightsEnabled: true,
      autoMinorVersionUpgrade: true,
    },
    { ...extra, protect: true }
  );

  return { dbCluster, dbInstance, dbSecurityGroup };
}

export function makeOpensearch(
  name: string,
  instanceCount: number,
  instanceType: string,
  config: { password: any; aliases?: string[]; vpc: awsx.ec2.Vpc; region: string }
) {
  const extra = { aliases: config.aliases?.map((name) => ({ name })) };
  const osSecurityGroup = new aws.ec2.SecurityGroup(
    name,
    {
      ingress: [{ fromPort: 443, toPort: 443, protocol: "tcp", cidrBlocks: ["0.0.0.0/0"] }],
      egress: [{ fromPort: 0, toPort: 0, protocol: "-1", cidrBlocks: ["0.0.0.0/0"] }],
      vpcId: config.vpc.vpcId,
    },
    { ...extra }
  );
  const osDomain = new aws.opensearch.Domain(
    name,
    {
      domainName: name,
      engineVersion: "OpenSearch_2.9",
      clusterConfig: {
        instanceType,
        instanceCount,
      },
      domainEndpointOptions: {
        enforceHttps: true,
        tlsSecurityPolicy: "Policy-Min-TLS-1-2-2019-07",
      },
      ebsOptions: {
        ebsEnabled: true,
        volumeSize: 100, // does this even matter?
        volumeType: "gp3",
      },
      encryptAtRest: {
        enabled: true,
      },
      nodeToNodeEncryption: {
        enabled: true,
      },
      vpcOptions: {
        subnetIds: config.vpc.privateSubnetIds.apply((ids) => ids.slice(0, 1)),
        securityGroupIds: [osSecurityGroup.id],
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
            Resource: `arn:aws:es:${config.region}:*:domain/${name}/*`,
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
          masterUserPassword: config.password,
        },
      },
    },
    { ...extra, protect: true }
  );
  return { osDomain, osSecurityGroup };
}

export function secretFrom(name: string, password: Output<string>, config: { key: string; provider: any }) {
  return new k8s.core.v1.Secret(
    name,
    {
      metadata: { namespace: "default" },
      type: "Opaque",
      data: {
        [config.key]: password.apply((password) => Buffer.from(password).toString("base64")),
      },
    },
    { provider: config.provider }
  );
}
