variable "environment" {
  description = "Deployment environment."
  type        = string

  validation {
    condition     = var.environment == "production"
    error_message = "Choose production."
  }
}

variable "database" {
  description = "The universe's PostgreSQL instance."
  type = object({
    instance_class = string
    multi_az       = bool
    major_version  = string
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

variable "mail_region" {
  description = "The region the global root's SES identity sends from."
  type        = string
}
