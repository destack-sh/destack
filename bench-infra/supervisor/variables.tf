#
# General
#

variable "bench_version" {
  type        = string
  description = "Bench version"
}

variable "git_commit" {
  type        = string
  description = "Git commit hash"
}

variable "env" {
  type        = string
  description = "Environment name"
}

variable "cloud" {
  type        = string
  description = "Cloud provider"
}

variable "region" {
  type        = string
  description = "Bench region"
}

variable "host_map" {
  type        = map(string)
  description = "Host map"
}

variable "supervisor_grpc_port" {
  type        = number
  description = "Supervisor GRPC port"
  default     = 60061
}

# 
# AWS
# 

variable "vpc_id" {
  type        = string
  description = "VPC ID"
}

variable "public_subnet_ids" {
  type        = list(string)
  description = "Public subnet IDs"
}

#
# DB
# 

variable "global_pg_url" {
  type        = string
  description = "URL for the global Postgres database"
}

#
# Kubernetes
# 

variable "image_pull_secret_name" {
  type        = string
  description = "Name of the image pull secret"
}

#
# Web
# 

variable "web_certificate_secret_name" {
  type        = string
  description = "Name of the cert secret"
}

variable "web_certificate_arn" {
  type        = string
  description = "Certificate ARN for the web domain"
}

variable "web_certificate_pem" {
  type        = string
  description = "Let's Encrypt certificate PEM for the web domain"
}

variable "web_certificate_private_key_pem" {
  type        = string
  description = "Let's Encrypt certificate private key PEM for the web domain"
}

# 
# 3rd party secrets
# 

variable "posthog_api_key" {
  type        = string
  description = "PostHog API key"
  sensitive   = true
}

variable "posthog_host" {
  type        = string
  description = "PostHog host"
}

variable "neon_api_key" {
  type        = string
  description = "Neon API key"
  sensitive   = true
}

variable "neon_base_url" {
  type        = string
  description = "Neon base URL"
  default     = "https://console.neon.tech/api/v2"
}
