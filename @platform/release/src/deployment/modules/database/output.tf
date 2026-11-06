output "database" {
  description = "Database and branch identifiers, excluding credentials."
  value = {
    name   = planetscale_postgres_branch.database.database
    branch = planetscale_postgres_branch.database.name
    id     = planetscale_postgres_branch.database.id
  }
}
output "role" {
  description = "The role its service connects as: its host, user and password."
  sensitive   = true
  value = {
    host     = planetscale_postgres_branch_role.owner.access_host_url
    user     = planetscale_postgres_branch_role.owner.username
    password = planetscale_postgres_branch_role.owner.password
  }
}
