import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as eks from "@pulumi/eks";
import * as k8s from "@pulumi/kubernetes";

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
