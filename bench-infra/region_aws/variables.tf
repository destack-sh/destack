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

# 
# DB
# 

variable "regional_pg_name" {
  type        = string
  description = "Name of the regional Postgres database"
  default     = "bench"
}

variable "regional_pg_username" {
  type        = string
  description = "Username for the regional Postgres database"
  default     = "bench"
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

variable "global_pg_url" {
  type        = string
  description = "URL for the global Postgres database"
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

variable "posthog_token" {
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

variable "openai_api_key" {
  type        = string
  description = "OpenAI API key"
  sensitive   = true
}

variable "gemini_api_key" {
  type        = string
  description = "Gemini API key"
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

variable "ip_api_key" {
  type        = string
  description = "IP Geolocation API key"
  sensitive   = true
}

variable "grafana_cloud_token" {
  type        = string
  description = "Grafana Cloud API token"
  sensitive   = true
}
