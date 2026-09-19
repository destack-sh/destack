variable "environment" {
  description = "Deployment environment."
  type        = string
  validation {
    condition     = contains(["development", "production"], var.environment)
    error_message = "Choose development or production."
  }
}
variable "database" {
  description = "PlanetScale Postgres deployment."
  type = object({
    enabled       = bool
    organization  = optional(string)
    region        = string
    cluster_size  = string
    major_version = string
  })
  validation {
    condition     = !var.database.enabled || try(length(var.database.organization) > 0, false)
    error_message = "An enabled database requires a PlanetScale organization."
  }
  validation {
    condition     = var.database.region == (var.residency == "eu" ? "eu-central" : "us-east")
    error_message = "The database region must match the deployment residency."
  }
}
variable "account_id" {
  description = "Cloudflare account containing regional infrastructure."
  type        = string
}
variable "residency" {
  description = "Storage jurisdiction."
  type        = string
  validation {
    condition     = contains(["eu", "us"], var.residency)
    error_message = "Choose eu or us."
  }
}
variable "package_import_id" {
  description = "Existing package bucket to adopt into this deployment's state."
  type        = string
  default     = null
  validation {
    condition     = var.package_import_id == null || var.package_import_id == "${var.account_id}/destack-${var.environment}-packages-${var.residency}/${var.residency}"
    error_message = "The imported bucket must match this deployment's account, environment, and residency."
  }
}
