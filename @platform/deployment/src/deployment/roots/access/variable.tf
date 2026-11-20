variable "aws_region" {
  description = "The platform account's home region."
  type        = string
}

variable "platform_profile" {
  description = "The AWS profile administering the platform account."
  type        = string
}

variable "management_profile" {
  description = "The AWS profile administering the organization's management account."
  type        = string
}

variable "identity_center_region" {
  description = "The region of the organization's Identity Center instance."
  type        = string
}

variable "platform_account_id" {
  description = "The account the platform runs in."
  type        = string
}

variable "management_account_id" {
  description = "The organization's management account."
  type        = string
}

variable "repository" {
  description = "The GitHub repository whose workflows plan and deploy the platform, as owner/name."
  type        = string
}

variable "people" {
  description = "The Identity Center user names of the people who plan, operate and administer the platform."
  type        = set(string)
}
