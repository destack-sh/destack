
#
# PostHog Reverse Proxy via CloudFront
#

# Variables
variable "posthog_subdomain" {
  type        = string
  description = "Subdomain for PostHog"
}

variable "posthog_region" {
  type        = string
  description = "PostHog Cloud region ('us' or 'eu')"
}

# PostHog origin domains
locals {
  posthog_api_domain    = "${var.posthog_region}.i.posthog.com"
  posthog_assets_domain = "${var.posthog_region}-assets.i.posthog.com"
  posthog_proxy_domain  = "${var.posthog_subdomain}.${local.main_website}"
}

# Cache policy for PostHog (origin-cors)
resource "aws_cloudfront_cache_policy" "posthog_origin_cors" {
  name        = "posthog-origin-cors-${var.env}"
  comment     = "Cache policy for PostHog with CORS headers"
  default_ttl = 0
  min_ttl     = 0
  max_ttl     = 31536000 # 1 year in seconds

  parameters_in_cache_key_and_forwarded_to_origin {
    enable_accept_encoding_brotli = true
    enable_accept_encoding_gzip   = true

    headers_config {
      header_behavior = "whitelist"
      headers {
        items = ["Origin", "Authorization"]
      }
    }

    query_strings_config {
      query_string_behavior = "all"
    }

    cookies_config {
      cookie_behavior = "all"
    }
  }
}

# Origin request policy for PostHog (origin-request-policy)
resource "aws_cloudfront_origin_request_policy" "posthog_origin_request" {
  name    = "posthog-origin-request-${var.env}"
  comment = "Origin request policy for PostHog Proxy"

  headers_config {
    header_behavior = "whitelist"
    headers {
      items = ["Origin", "Host"]
    }
  }

  query_strings_config {
    query_string_behavior = "all"
  }

  cookies_config {
    cookie_behavior = "all"
  }
}

# Response headers policy (CORS-with-preflight)
resource "aws_cloudfront_response_headers_policy" "posthog_cors" {
  name    = "posthog-cors-${var.env}"
  comment = "CORS policy for PostHog Proxy"

  cors_config {
    access_control_allow_credentials = true
    access_control_allow_headers {
      items = ["Origin", "Authorization", "Content-Type", "Accept"]
    }
    access_control_allow_methods {
      items = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
    }
    access_control_allow_origins {
      items = ["https://${local.main_website}", "https://*.${local.main_website}"]
    }
    access_control_max_age_sec = 600
    origin_override            = true
  }
}

# CloudFront distribution
resource "aws_cloudfront_distribution" "posthog_proxy" {
  enabled             = true
  is_ipv6_enabled     = true
  comment             = "PostHog Proxy for ${var.env}"
  default_root_object = "/"
  price_class         = "PriceClass_200"
  aliases             = [local.posthog_proxy_domain]

  # Main PostHog API origin
  origin {
    domain_name = local.posthog_api_domain
    origin_id   = "posthog-api"

    custom_origin_config {
      http_port              = 80
      https_port             = 443
      origin_protocol_policy = "https-only"
      origin_ssl_protocols   = ["TLSv1.2"]
    }
  }

  # PostHog static assets origin
  origin {
    domain_name = local.posthog_assets_domain
    origin_id   = "posthog-assets"

    custom_origin_config {
      http_port              = 80
      https_port             = 443
      origin_protocol_policy = "https-only"
      origin_ssl_protocols   = ["TLSv1.2"]
    }
  }

  # Default behavior - route to PostHog API
  default_cache_behavior {
    allowed_methods  = ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"]
    cached_methods   = ["GET", "HEAD", "OPTIONS"]
    target_origin_id = "posthog-api"

    cache_policy_id            = aws_cloudfront_cache_policy.posthog_origin_cors.id
    origin_request_policy_id   = aws_cloudfront_origin_request_policy.posthog_origin_request.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.posthog_cors.id

    compress               = true
    viewer_protocol_policy = "redirect-to-https"
  }

  # Static assets behavior pattern
  ordered_cache_behavior {
    path_pattern     = "/static/*"
    allowed_methods  = ["GET", "HEAD", "OPTIONS"]
    cached_methods   = ["GET", "HEAD", "OPTIONS"]
    target_origin_id = "posthog-assets"

    cache_policy_id            = aws_cloudfront_cache_policy.posthog_origin_cors.id
    origin_request_policy_id   = aws_cloudfront_origin_request_policy.posthog_origin_request.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.posthog_cors.id

    compress               = true
    viewer_protocol_policy = "redirect-to-https"
  }

  viewer_certificate {
    acm_certificate_arn      = aws_acm_certificate.main_website.arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2019"
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  tags = {
    Name        = "posthog-proxy-${var.env}"
    Environment = var.env
  }
}

# Cloudflare DNS record for PostHog proxy
resource "cloudflare_record" "posthog_proxy" {
  zone_id = data.cloudflare_zone.main_website.id
  name    = var.posthog_subdomain
  type    = "CNAME"
  content = aws_cloudfront_distribution.posthog_proxy.domain_name
  ttl     = 300
  proxied = false
}

# Output the PostHog proxy URL
output "posthog_proxy_url" {
  value = "https://${local.posthog_proxy_domain}"
}
