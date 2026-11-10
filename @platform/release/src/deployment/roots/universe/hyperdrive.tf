# trust the region's RDS authority the database is pinned to, so Hyperdrive verifies its certificate and host
resource "cloudflare_mtls_certificate" "database" {
  account_id   = var.account_id
  name         = "destack-${var.environment}-universe-database-${substr(sha256(module.database.authority), 0, 8)}"
  ca           = true
  certificates = module.database.authority

  # replace it before deleting it, as Hyperdrive refuses losing a certificate in use, named by its content as names are unique
  lifecycle {
    create_before_destroy = true
  }
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
