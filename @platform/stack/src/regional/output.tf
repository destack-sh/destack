output "database" {
  description = "Provisioned database and branch identifiers."
  value       = one(values(module.database)[*].database)
}
output "packages" {
  description = "Regional package bucket."
  value       = cloudflare_r2_bucket.packages.name
}
output "files" {
  description = "Regional application file bucket."
  value       = cloudflare_r2_bucket.files.name
}
