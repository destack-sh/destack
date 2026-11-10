output "hyperdrive" {
  description = "The Hyperdrive configuration the universe Worker connects through."
  value       = cloudflare_hyperdrive_config.universe.id
}

output "account" {
  description = "The Cloudflare account running the universe Worker."
  value       = var.account_id
}

output "files" {
  description = "The bucket keeping the universe's files: its call events' segments and its blobs."
  value       = aws_s3_bucket.files.bucket
}

output "files_region" {
  description = "The region of the universe's files bucket."
  value       = aws_s3_bucket.files.region
}

output "kms_key_arn" {
  description = "The KMS key sealing the universe's root keys."
  value       = module.kms.key_arn
}

output "kms_region" {
  description = "The region of the universe's KMS key."
  value       = module.kms.region
}

output "access_key_id" {
  description = "The universe Worker's access key id."
  value       = module.worker.access_key_id
}

output "secret_access_key" {
  description = "The universe Worker's secret access key."
  value       = module.worker.secret_access_key
  sensitive   = true
}
