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

variable "is_primary" {
  type        = bool
  description = "Whether this is the primary region (with the supervisor)"
}

variable "host_map" {
  type        = map(string)
  description = "Host map"
}

variable "aws_availability_zones" {
  type        = list(string)
  description = "AWS availability zones"
}

variable "host_grpc_port" {
  type        = number
  description = "Host GRPC port"
  default     = 60061
}

variable "supervisor_url" {
  type        = string
  description = "Supervisor URL"
}


#
# AWS
# 

variable "vpc_network_cidr" {
  type        = string
  description = "CIDR block for the VPC"
}

variable "system_min_cluster_size" {
  type        = number
  description = "Minimum size of the EKS cluster"
}

variable "system_max_cluster_size" {
  type        = number
  description = "Maximum size of the EKS cluster"
}

variable "system_desired_cluster_size" {
  type        = number
  description = "Desired size of the EKS cluster"
}

variable "system_node_instance_type" {
  type        = string
  description = "EC2 instance types for EKS nodes"
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
# Web
# 

variable "web_zone_id" {
  type        = string
  description = "Cloudflare zone ID for the web domain"
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

variable "openai_api_key" {
  type        = string
  description = "OpenAI API key"
  sensitive   = true
}

variable "anthropic_api_key" {
  type        = string
  description = "Anthropic API key"
  sensitive   = true
}

variable "ghcr_username" {
  type        = string
  description = "GitHub Container Registry username"
}
variable "ghcr_token" {
  type        = string
  description = "GitHub Container Registry token"
  sensitive   = true
}

variable "betterstack_token" {
  type        = string
  description = "BetterStack source token"
  sensitive   = true
}

