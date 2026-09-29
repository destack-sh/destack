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
}
