variable "environment" {
  description = "Deployment environment."
  type        = string
  validation {
    condition     = var.environment == "production"
    error_message = "Choose production."
  }
}
variable "account_id" {
  description = "Cloudflare account containing residency infrastructure."
  type        = string
}
variable "residency" {
  description = "The residency's code, which is also its storage jurisdiction."
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
variable "machine" {
  description = "The name of the residency's cloud machine, one label of the platform account's machines."
  type        = string
  validation {
    condition     = can(regex("^[a-z0-9]([a-z0-9-]*[a-z0-9])?$", var.machine)) && !strcontains(var.machine, "--")
    error_message = "A machine name is a DNS label without --."
  }
}
variable "machine_id" {
  description = "The identifier of the residency's cloud machine, the owner its sealed keys decrypt for."
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
