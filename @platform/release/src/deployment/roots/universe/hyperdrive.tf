# trust the region's RDS authorities, so Hyperdrive verifies the database's certificate and host
resource "cloudflare_mtls_certificate" "database" {
  account_id   = var.account_id
  name         = "destack-${var.environment}-universe-database"
  ca           = true
  certificates = module.database.authority
}

# pool the universe Worker's connections to its database at the edge, reading every query fresh
resource "cloudflare_hyperdrive_config" "universe" {
  account_id = var.account_id
  name       = "destack-${var.environment}-universe"
  caching = {
    disabled = true
  }
  origin = {
    scheme   = "postgres"
    host     = module.database.role.host
    port     = 5432
    database = "postgres"
    user     = module.database.role.user
    password = module.database.role.password
  }
  mtls = {
    ca_certificate_id = cloudflare_mtls_certificate.database.id
    sslmode           = "verify-full"
  }
}
