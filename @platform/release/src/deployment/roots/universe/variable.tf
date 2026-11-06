variable "environment" {
  description = "Deployment environment."
  type        = string
  validation {
    condition     = var.environment == "production"
    error_message = "Choose production."
  }
}
variable "database" {
  description = "PlanetScale Postgres deployment."
  type = object({
    organization  = string
    region        = string
    cluster_size  = string
    major_version = string
  })
}
variable "account_id" {
  description = "Cloudflare account running the universe Worker."
  type        = string
}
variable "kms_operators" {
  description = "The roles that create, repolicy, grant and replicate the root key, as ARN patterns."
  type        = list(string)
}
