module "kms" {
  source    = "../../modules/kms"
  operators = var.kms_operators
  name      = "destack-${var.environment}-universe"
  owner     = "universe"
  worker    = module.worker.arn
}
