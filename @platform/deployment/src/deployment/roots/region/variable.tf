variable "environment" {
  description = "Deployment environment."
  type        = string
  validation {
    condition     = var.environment == "production"
    error_message = "Choose production."
  }
}
variable "account_id" {
  description = "Cloudflare account containing region infrastructure."
  type        = string
}
variable "region" {
  description = "The region's code, which is also its storage jurisdiction."
  type        = string
  validation {
    condition     = contains(["eu", "us"], var.region)
    error_message = "Choose eu or us."
  }
}
variable "machine" {
  description = "The name of the region's managed machine, one label of the platform account's machines."
  type        = string
  validation {
    condition     = can(regex("^[a-z0-9]([a-z0-9-]*[a-z0-9])?$", var.machine)) && !strcontains(var.machine, "--")
    error_message = "A machine name is a DNS label without --."
  }
}
variable "machine_id" {
  description = "The identifier of the region's managed machine, the owner its sealed keys decrypt for."
  type        = string
  validation {
    condition     = can(regex("^machine-[0-9a-f-]{36}$", var.machine_id))
    error_message = "A machine identifier is machine- and a UUID."
  }
}
variable "kms_operators" {
  description = "The roles that create, repolicy, grant and replicate the root key, as ARN patterns."
  type        = list(string)
}

variable "mail_region" {
  description = "The region the global root's SES identity sends from."
  type        = string
}
