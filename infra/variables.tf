#
# General
# NOTE regions must be added manually to main.tf :StaticRegions
#

variable "env" {
  type        = string
  description = "Environment name"
}

variable "global_region" {
  type        = string
  description = "Primary AWS region"
  default     = "eu-central-1"
}

variable "regions" {
  type        = list(string)
  description = "Regions to create"
  default     = ["eu-central-1"]
}

variable "region_availability_zones" {
  type        = map(list(string))
  description = "Availability zones for the regional VPCs"
  default = {
    "eu-central-1" = ["eu-central-1a", "eu-central-1b"]
  }
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

variable "global_vpc_network_cidr" {
  type        = string
  description = "CIDR block for the global VPC"
  default     = "10.0.0.0/16"
}

variable "region_vpc_network_cidrs" {
  type        = map(string)
  description = "CIDR blocks for the regional VPCs"
  default = {
    "eu-central-1" = "10.1.0.0/16"
  }
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

variable "cors_allowed_hosts" {
  type        = string
  description = "Allowed hosts"
}

variable "cors_allowed_origins" {
  type        = string
  description = "Allowed origins"
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

variable "ghcr_token" {
  type        = string
  description = "GitHub Container Registry token"
  sensitive   = true
}

