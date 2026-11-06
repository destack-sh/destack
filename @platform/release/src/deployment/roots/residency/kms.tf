module "kms" {
  source    = "../../modules/kms"
  operators = var.kms_operators
  name      = "destack-${var.environment}-machine-${var.residency}"
  owner     = "machine ${var.machine_id}"
  providers = {
    aws         = aws
    aws.replica = aws.replica
  }
}
