resource "planetscale_postgres_branch_role" "process" {
  for_each     = var.processes
  organization = var.organization
  database     = planetscale_postgres_branch.database.database
  branch       = planetscale_postgres_branch.database.name
  name         = each.key
}
