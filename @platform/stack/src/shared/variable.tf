variable "account_id" {
  description = "Cloudflare account containing shared infrastructure."
  type        = string
}

variable "aws_region" {
  description = "AWS region sending Destack mail through SES."
  type        = string
  default     = "eu-central-1"
}

variable "dmarc_report_address" {
  description = "Address receiving aggregate DMARC reports for destack.app."
  type        = string
  validation {
    condition     = can(regex("^[^@\\s]+@[^@\\s]+$", var.dmarc_report_address))
    error_message = "Use one mail address."
  }
}
