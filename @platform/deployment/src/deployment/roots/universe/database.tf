# accept connections from Cloudflare's network
data "cloudflare_ip_ranges" "cloudflare" {}

module "database" {
  source         = "../../modules/database"
  name           = "destack-${var.environment}-universe"
  instance_class = var.database.instance_class
  multi_az       = var.database.multi_az
  major_version  = var.database.major_version
  role           = "universe"
  clients        = data.cloudflare_ip_ranges.cloudflare.ipv4_cidrs
}
