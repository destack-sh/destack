# let the service's one role own its schema, creating and migrating its tables as the database's owner does
resource "planetscale_postgres_branch_role" "owner" {
  organization    = var.organization
  database        = planetscale_postgres_branch.database.database
  branch          = planetscale_postgres_branch.database.name
  name            = var.role
  inherited_roles = ["postgres"]
}
