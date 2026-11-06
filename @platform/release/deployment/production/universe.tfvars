environment = "production"
account_id  = "27c0d00fb3a27a4ccbf46a3cceab9301"

database = {
  organization  = "symbol-industries"
  region        = "eu-central"
  cluster_size  = "PS_5_AWS_ARM"
  major_version = "17"
}

kms_operators = [
  "arn:aws:iam::903685415634:role/aws-reserved/sso.amazonaws.com/AWSReservedSSO_destack-platform-deploy_bfb3799400e37415",
]
