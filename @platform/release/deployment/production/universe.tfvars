environment = "production"
account_id  = "27c0d00fb3a27a4ccbf46a3cceab9301"
mail_region = "eu-central-2"

database = {
  instance_class = "db.t4g.small"
  multi_az       = true
  major_version  = "17"
}

kms_operators = [
  "arn:aws:iam::903685415634:role/aws-reserved/sso.amazonaws.com/AWSReservedSSO_destack-platform-operator_*",
  "arn:aws:iam::903685415634:role/aws-reserved/sso.amazonaws.com/AWSReservedSSO_AdministratorAccess_*",
]
