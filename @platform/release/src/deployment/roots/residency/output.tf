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
output "s3" {
  description = "The name below which the cloud machine answers its buckets over S3."
  value       = local.s3
}
output "account" {
  description = "The Cloudflare account running the machine Worker."
  value       = var.account_id
}

output "r2_access_key_id" {
  description = "The cloud machine's R2 access key id: its token's id."
  value       = cloudflare_account_token.machine.id
}

output "r2_secret_access_key" {
  description = "The cloud machine's R2 secret access key: the SHA-256 of its token's value."
  value       = sha256(cloudflare_account_token.machine.value)
  sensitive   = true
}

output "kms_key_arn" {
  description = "The KMS key sealing the cloud machine's keys."
  value       = module.kms.key_arn
}
output "kms_region" {
  description = "The region of the cloud machine's KMS key."
  value       = module.kms.region
}
output "kms_access_key_id" {
  description = "The cloud machine Worker's access key id, decrypting its own keys alone."
  value       = module.kms.access_key_id
}
output "kms_secret_access_key" {
  description = "The cloud machine Worker's secret access key."
  value       = module.kms.secret_access_key
  sensitive   = true
}
