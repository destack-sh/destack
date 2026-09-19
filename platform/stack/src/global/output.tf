output "database" {
  description = "Provisioned database and branch identifiers."
  value       = one(values(module.database)[*].database)
}
