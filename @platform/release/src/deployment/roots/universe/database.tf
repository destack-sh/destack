module "database" {
  source        = "../../modules/database"
  organization  = var.database.organization
  name          = "destack-${var.environment}-global"
  region        = var.database.region
  cluster_size  = var.database.cluster_size
  major_version = var.database.major_version
  role          = "universe"
}
