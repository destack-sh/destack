# pool the universe Worker's connections to its database at the edge
resource "cloudflare_hyperdrive_config" "universe" {
  account_id = var.account_id
  name       = "destack-${var.environment}-universe"
  origin = {
    scheme   = "postgres"
    host     = module.database.role.host
    port     = 5432
    database = "postgres"
    user     = module.database.role.user
    password = module.database.role.password
  }
}
