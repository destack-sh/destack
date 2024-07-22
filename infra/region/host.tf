#
# Host
# 

locals {
  host_env_vars = {
    SERVICE_NAME = "host"
    ENVIRONMENT  = var.env
    CLOUD        = var.cloud
    REGION       = var.region

    GLOBAL_PG_HOST       = var.global_pg_host
    GLOBAL_PG_NAME       = var.global_pg_name
    GLOBAL_PG_USERNAME   = var.global_pg_username
    GLOBAL_PG_PASSWORD   = var.global_pg_password
    GLOBAL_PG_CRYPTO_KEY = var.global_pg_crypto_key

    TRACING   = 0
    LOG_LEVEL = "DEBUG"
    LOG_MODE  = "JSON"

    SENTRY_DSN        = var.sentry_dsn
    NEON_API_KEY      = var.neon_api_key
    NEON_BASE_URL     = var.neon_base_url
    OPENAI_API_KEY    = var.openai_api_key
    ANTHROPIC_API_KEY = var.anthropic_api_key
    GHCR_TOKEN        = var.ghcr_token
  }
}
