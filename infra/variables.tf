#
# General
#

variable "env" {
  type        = string
  description = "Environment name"
}

#
# AWS
# 

variable "aws_region" {
  type        = string
  description = "AWS region for resources"
}

variable "aws_availability_zones" {
  type        = list(string)
  description = "Availability zones for the VPC"
}

variable "vpc_network_cidr" {
  type        = string
  description = "CIDR block for the VPC"
  default     = "10.0.0.0/16"
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
# API
# 

variable "cors_allowed_hosts" {
  type        = string
  description = "Allowed hosts"
  default     = "*"
}

variable "cors_allowed_origins" {
  type        = string
  description = "Allowed origins"
  default     = "*"
}

variable "webapp_url" {
  type        = string
  description = "URL for the web application"
}

# 
# 3rd party secrets
# 

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

