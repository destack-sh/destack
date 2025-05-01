#
# General
# NOTE regions must be added manually to main.tf :StaticRegions
#

variable "env" {
  type        = string
  description = "Environment name"
}

#
# Cloudflare
# 

variable "cloudflare_api_token" {
  type        = string
  description = "Cloudflare API key"
  sensitive   = true
}

# 
# AWS
# 

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

variable "openrouter_api_key" {
  type        = string
  description = "OpenRouter API key"
  sensitive   = true
}

variable "xai_api_key" {
  type        = string
  description = "xAI API key"
  sensitive   = true
}

variable "exa_api_key" {
  type        = string
  description = "Exa API key"
  sensitive   = true
}

variable "unsplash_access_key" {
  type        = string
  description = "Unsplash access key"
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

variable "ip_api_key" {
  type        = string
  description = "IP Geolocation API key"
  sensitive   = true
}
