resource "planetscale_postgres_branch_role" "owner" {
  organization = var.organization
  database     = planetscale_postgres_branch.database.database
  branch       = planetscale_postgres_branch.database.name
  name         = var.role
}
