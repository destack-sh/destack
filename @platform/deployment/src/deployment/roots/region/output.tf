output "bucket" {
  description = "The region's bucket, keyed by owner."
  value       = cloudflare_r2_bucket.region.name
}
output "host" {
  description = "The managed machine's name, which its Worker's routes answer."
  value       = local.host
}
output "account" {
  description = "The Cloudflare account running the machine Worker."
  value       = var.account_id
}

output "kms_key_arn" {
  description = "The KMS key sealing the managed machine's keys."
  value       = module.kms.key_arn
}
output "kms_region" {
  description = "The region of the managed machine's KMS key."
  value       = module.kms.region
}
output "access_key_id" {
  description = "The managed machine Worker's access key id."
  value       = module.worker.access_key_id
}

output "secret_access_key" {
  description = "The managed machine Worker's secret access key."
  value       = module.worker.secret_access_key
  sensitive   = true
}
