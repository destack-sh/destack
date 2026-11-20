environment       = "production"
account_id        = "27c0d00fb3a27a4ccbf46a3cceab9301"
residency         = "eu"
package_import_id = "27c0d00fb3a27a4ccbf46a3cceab9301/destack-production-packages-eu/eu"
machine           = "eu-central"
machine_id        = "machine-01a11e60-7834-77f0-ad65-3470b05f0b39"
mail_region       = "eu-central-2"

database = {
  instance_class = "db.t4g.small"
  multi_az       = true
  major_version  = "17"
}

kms_operators = [
  "arn:aws:iam::903685415634:role/aws-reserved/sso.amazonaws.com/AWSReservedSSO_destack-platform-operator_*",
  "arn:aws:iam::903685415634:role/aws-reserved/sso.amazonaws.com/AWSReservedSSO_AdministratorAccess_*",
]
