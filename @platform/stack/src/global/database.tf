module "database" {
  source   = "../database"
  for_each = var.database.enabled ? toset(["main"]) : toset([])
  providers = {
    planetscale = planetscale.database[each.key]
  }
  organization  = var.database.organization
  name          = "destack-${var.environment}-global"
  region        = var.database.region
  cluster_size  = var.database.cluster_size
  major_version = var.database.major_version
}
