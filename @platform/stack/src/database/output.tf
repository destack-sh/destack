output "database" {
  description = "Database and branch identifiers, excluding credentials."
  value = {
    name   = planetscale_postgres_branch.database.database
    branch = planetscale_postgres_branch.database.name
    id     = planetscale_postgres_branch.database.id
  }
}
