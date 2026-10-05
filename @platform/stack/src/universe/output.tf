output "database" {
  description = "Provisioned database and branch identifiers."
  value       = one(values(module.database)[*].database)
}
output "processes" {
  description = "Each process's Hyperdrive configuration, connecting it as its own role."
  value       = { for name, config in cloudflare_hyperdrive_config.process : name => config.id }
}
output "account" {
  description = "The Cloudflare account running the processes' Workers."
  value       = var.account_id
}
output "roles" {
  description = "Each process's database role, which releases migrate as."
  sensitive   = true
  value       = one(values(module.database)[*].roles)
}
