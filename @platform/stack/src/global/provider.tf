terraform {
  required_version = ">= 1.12.0, < 2.0.0"
  required_providers {
    planetscale = {
      source  = "planetscale/planetscale"
      version = "1.11.0"
    }
  }
}
provider "planetscale" {
  alias    = "database"
  for_each = var.database.organization != null ? toset(["main"]) : toset([])
}
