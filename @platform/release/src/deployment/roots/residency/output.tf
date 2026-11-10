output "packages" {
  description = "Residency package bucket."
  value       = cloudflare_r2_bucket.packages.name
}
output "files" {
  description = "Residency application file bucket."
  value       = cloudflare_r2_bucket.files.name
}
output "host" {
  description = "The cloud machine's name, which its Worker's routes answer."
  value       = local.host
}
output "account" {
  description = "The Cloudflare account running the machine Worker."
  value       = var.account_id
}

output "kms_key_arn" {
  description = "The KMS key sealing the cloud machine's keys."
  value       = module.kms.key_arn
}
output "kms_region" {
  description = "The region of the cloud machine's KMS key."
  value       = module.kms.region
}
output "access_key_id" {
  description = "The cloud machine Worker's access key id."
  value       = module.worker.access_key_id
}

output "secret_access_key" {
  description = "The cloud machine Worker's secret access key."
  value       = module.worker.secret_access_key
  sensitive   = true
}

output "hyperdrive" {
  description = "The Hyperdrive configuration the machine Worker reaches the residency's database through."
  value       = cloudflare_hyperdrive_config.residency.id
}
