
# database credentials
resource "kubernetes_secret" "db_secret" {
  metadata {
    name = "${local.prefix}-db-credentials"
  }

  data = {
    # Database URLs
    GLOBAL_PG_URL = var.global_pg_url
    REGIONAL_PG_MAP = join(",", flatten([
      for k, v in {
        "${var.region}" = "postgresql://${var.regional_pg_username}:${random_password.regional_pg_password.result}@${aws_rds_cluster.regional_pg_primary.endpoint}/${var.regional_pg_name}"
        } : [
        format("%s=%s", k, v)
      ]
    ]))
  }
}

# external keys
resource "kubernetes_secret" "external_secret" {
  metadata {
    name = "${local.prefix}-external-api-keys"
  }

  data = {
    NEON_API_KEY        = var.neon_api_key
    NEON_BASE_URL       = var.neon_base_url
    OPENAI_API_KEY      = var.openai_api_key
    ANTHROPIC_API_KEY   = var.anthropic_api_key
    OPENROUTER_API_KEY  = var.openrouter_api_key
    XAI_API_KEY         = var.xai_api_key
    EXA_API_KEY         = var.exa_api_key
    POSTHOG_TOKEN       = var.posthog_token
    POSTHOG_HOST        = var.posthog_host
    UNSPLASH_ACCESS_KEY = var.unsplash_access_key
    GHCR_TOKEN          = var.ghcr_token
  }
}
