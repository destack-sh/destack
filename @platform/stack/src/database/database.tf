resource "planetscale_postgres_branch" "database" {
  organization       = var.organization
  database           = var.name
  name               = "main"
  region             = var.region
  cluster_size       = var.cluster_size
  major_version      = var.major_version
  deletion_protected = true
  lifecycle {
    prevent_destroy = true
  }
}
