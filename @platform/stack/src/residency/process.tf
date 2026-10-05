resource "cloudflare_hyperdrive_config" "process" {
  for_each   = var.database.enabled ? var.processes : {}
  account_id = var.account_id
  name       = "destack-${var.environment}-${each.key}"
  origin = {
    scheme   = "postgres"
    host     = module.database["main"].roles[each.key].host
    port     = 5432
    database = "postgres"
    user     = module.database["main"].roles[each.key].user
    password = module.database["main"].roles[each.key].password
  }
}
