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

variable "global_pg_host" {
  type        = string
  description = "Host for the global Postgres database"
}

variable "global_pg_name" {
  type        = string
  description = "Name of the global Postgres database"
}

variable "global_pg_username" {
  type        = string
  description = "Username for the global Postgres database"
}

variable "global_pg_password" {
  type        = string
  description = "Password for the global Postgres database"
  sensitive   = true
}

variable "global_pg_crypto_key" {
  type        = string
  description = "Crypto key for the global Postgres database"
  sensitive   = true
}

#
# Kubernetes
# 

variable "image_pull_secret_name" {
  type        = string
  description = "Name of the image pull secret"
}

variable "web_certificate_secret_name" {
  type        = string
  description = "Name of the cert secret"
}

#
# Web
# 

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

variable "sentry_dsn" {
  type        = string
  description = "Sentry DSN"
  sensitive   = true
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
