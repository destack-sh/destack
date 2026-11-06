output "database" {
  description = "Provisioned database and branch identifiers."
  value       = module.database.database
}
output "hyperdrive" {
  description = "The Hyperdrive configuration the universe Worker connects through."
  value       = cloudflare_hyperdrive_config.universe.id
}
output "account" {
  description = "The Cloudflare account running the universe Worker."
  value       = var.account_id
}
output "role" {
  description = "The universe's database role, which releases migrate as."
  sensitive   = true
  value       = module.database.role
}
output "history" {
  description = "The bucket keeping the account service's history."
  value       = cloudflare_r2_bucket.history.name
}
output "kms_key_arn" {
  description = "The KMS key sealing the universe's root keys."
  value       = module.kms.key_arn
}
output "kms_region" {
  description = "The region of the universe's KMS key."
  value       = module.kms.region
}
output "kms_access_key_id" {
  description = "The universe Worker's access key id, decrypting its own root keys alone."
  value       = module.kms.access_key_id
}
output "kms_secret_access_key" {
  description = "The universe Worker's secret access key."
  value       = module.kms.secret_access_key
  sensitive   = true
}
