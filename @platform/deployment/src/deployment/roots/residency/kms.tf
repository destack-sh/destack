module "kms" {
  source    = "../../modules/kms"
  operators = var.kms_operators
  name      = "destack-${var.environment}-machine-${var.residency}"
  owner     = "machine ${var.machine_id}"
  worker    = module.worker.arn
}

# replicate the key inside the residency
resource "aws_kms_replica_key" "root" {
  provider                = aws.replica
  description             = "The replica of the root keys of destack-${var.environment}-machine-${var.residency}."
  primary_key_arn         = module.kms.key_arn
  deletion_window_in_days = 30
  policy                  = module.kms.policy
  tags                    = { "destack-root" = "destack-${var.environment}-machine-${var.residency}" }
}
