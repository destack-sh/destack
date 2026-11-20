# accept connections from Cloudflare's network
data "cloudflare_ip_ranges" "cloudflare" {}

# keep the residency home's main database in the residency
module "database" {
  source         = "../../modules/database"
  name           = "destack-${var.environment}-residency-${var.residency}"
  instance_class = var.database.instance_class
  multi_az       = var.database.multi_az
  major_version  = var.database.major_version
  role           = "residency"
  clients        = data.cloudflare_ip_ranges.cloudflare.ipv4_cidrs
}

# trust the region's RDS authority the database is pinned to, so Hyperdrive verifies its certificate and host
resource "cloudflare_mtls_certificate" "database" {
  account_id   = var.account_id
  name         = "destack-${var.environment}-residency-${var.residency}-database-${substr(sha256(module.database.authority), 0, 8)}"
  ca           = true
  certificates = module.database.authority

  # replace it before deleting it, as Hyperdrive refuses losing a certificate in use, named by its content as names are unique
  lifecycle {
    create_before_destroy = true
  }
}

# pool the machine Worker's connections without caching
resource "cloudflare_hyperdrive_config" "residency" {
  account_id = var.account_id
  name       = "destack-${var.environment}-residency-${var.residency}"
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
