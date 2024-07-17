#
# General
#

variable "version" {
  type        = string
  description = "Version of Bench"
}

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

variable "min_cluster_size" {
  type        = number
  description = "Minimum size of the EKS cluster"
}

variable "max_cluster_size" {
  type        = number
  description = "Maximum size of the EKS cluster"
}

variable "desired_cluster_size" {
  type        = number
  description = "Desired size of the EKS cluster"
}

variable "vpc_network_cidr" {
  type        = string
  description = "CIDR block for the VPC"
}

variable "eks_node_instance_type" {
  type        = string
  description = "EC2 instance type for EKS nodes"
}

#
# DB
# 

variable "global_pg_name" {
  type        = string
  description = "Name of the global Postgres database"
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
}

variable "cors_allowed_origins" {
  type        = string
  description = "Allowed origins"
}

variable "webapp_url" {
  type        = string
  description = "URL for the web application"
}

# 
# 3rd party secrets
# 

variable "sentry_dsn" {
  type        = string
  description = "Sentry DSN for error tracking"
  sensitive   = true
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

variable "betterstack_secret" {
  type        = string
  description = "BetterStack secret token"
  sensitive   = true
}
