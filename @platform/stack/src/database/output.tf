output "database" {
  description = "Database and branch identifiers, excluding credentials."
  value = {
    name   = planetscale_postgres_branch.database.database
    branch = planetscale_postgres_branch.database.name
    id     = planetscale_postgres_branch.database.id
  }
}
output "roles" {
  description = "Each process's role: its host, user and password."
  sensitive   = true
  value = {
    for name, role in planetscale_postgres_branch_role.process : name => {
      host     = role.access_host_url
      user     = role.username
      password = role.password
    }
  }
}
