terraform {
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
  }
}

provider "aws" {
  alias  = "us-east-1"
  region = "us-east-1"
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}

data "cloudflare_zone" "justbench_com" {
  name = "justbench.com"
}

# 
# ACM certificate
#

resource "aws_acm_certificate" "justbench_com" {
  domain_name               = "justbench.com"
  subject_alternative_names = ["*.justbench.com"]
  validation_method         = "DNS"

  provider = aws.us-east-1 // all ACM certificates must be in us-east-1

  tags = {
    Name = "bench-${var.env}-global-web-cert"
  }
}

locals {
  # deduplicate domain validation options (may go in the same record)
  domain_validation_options = toset([
    for dvo in aws_acm_certificate.justbench_com.domain_validation_options : {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  ])
}
resource "cloudflare_record" "cert_validation" {
  for_each = {
    for dvo in local.domain_validation_options :
    dvo.name => dvo
  }

  zone_id = data.cloudflare_zone.justbench_com.id
  name    = each.value.name
  type    = each.value.type
  value   = each.value.record
  ttl     = 60
}

#
# S3 bucket for bench-web
#

resource "aws_s3_bucket" "bench_web" {
  bucket = "bench-${var.env}-global-web-public"
}

# make the S3 bucket public (for read access)
resource "aws_s3_bucket_public_access_block" "bench_web" {
  bucket = aws_s3_bucket.bench_web.id

  block_public_acls       = false
  block_public_policy     = false
  ignore_public_acls      = false
  restrict_public_buckets = false
}
resource "aws_s3_bucket_ownership_controls" "bench_web" {
  bucket = aws_s3_bucket.bench_web.id
  rule {
    object_ownership = "BucketOwnerPreferred"
  }
}
resource "aws_s3_bucket_acl" "bench_web" {
  bucket     = aws_s3_bucket.bench_web.id
  acl        = "public-read"
  depends_on = [aws_s3_bucket_public_access_block.bench_web]
}
resource "aws_s3_bucket_policy" "bench_web_allow_public" {
  bucket = aws_s3_bucket.bench_web.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "PublicReadGetObject"
        Effect = "Allow"
        Principal = {
          "AWS" : "*"
        }
        Action   = "s3:GetObject"
        Resource = "arn:aws:s3:::${aws_s3_bucket.bench_web.bucket}/*"
      },
    ]
  })
  depends_on = [aws_s3_bucket_public_access_block.bench_web]
}

# upload the built bench-web/dist to the S3 bucket
locals {
  # :BenchWebEnv
  web_variables = {
    "VITE_APP_COMMIT"      = data.external.git.result.sha
    "VITE_APP_ENVIRONMENT" = var.env
    "VITE_APP_SENTRY_DSN"  = var.sentry_dsn
  }
  web_variables_subs = [for k, v in local.web_variables : {
    regex = "/[a-zA-Z0-9]+\\.${k}/",
    sub   = "\"${v}\""
  }]
  web_files_unfiltered = fileset("../bench-web/dist", "**")
  web_exclude_files = [
    ".DS_Store",
    "other_file_to_exclude"
  ]
  web_files = setsubtract(local.web_files_unfiltered, local.web_exclude_files)
}
resource "aws_s3_object" "bench_web_files" {
  for_each = local.web_files

  bucket = aws_s3_bucket.bench_web.bucket
  key    = each.key
  content_type = lookup({
    "html" = "text/html",
    "js"   = "application/javascript",
    "css"  = "text/css",
    "png"  = "image/png",
    "jpg"  = "image/jpeg",
    "svg"  = "image/svg+xml",
    "json" = "application/json"
  }, split(".", each.key)[length(split(".", each.key)) - 1], "application/octet-stream")

  content_base64 = endswith(each.key, ".js") ? base64encode(
    # :BenchWebEnv (one replace for each variable.. :Cleanup)
    replace(
      replace(
        replace(
          file("../bench-web/dist/${each.key}"),
          local.web_variables_subs[0].regex,
          local.web_variables_subs[0].sub
        ),
        local.web_variables_subs[1].regex,
        local.web_variables_subs[1].sub
      ),
      local.web_variables_subs[2].regex,
      local.web_variables_subs[2].sub
    )
  ) : filebase64("../bench-web/dist/${each.key}")

  tags = {
    Name        = "bench-${var.env}-web"
    Environment = var.env
  }
}

#
# CloudFront distribution
#

resource "aws_cloudfront_distribution" "bench_web" {
  origin {
    domain_name = aws_s3_bucket.bench_web.bucket_regional_domain_name
    origin_id   = aws_s3_bucket.bench_web.bucket
  }

  enabled             = true
  is_ipv6_enabled     = true
  comment             = "bench-${var.env}-global-web"
  default_root_object = "index.html"

  aliases = ["justbench.com"]

  default_cache_behavior {
    allowed_methods  = ["GET", "HEAD", "OPTIONS"]
    cached_methods   = ["GET", "HEAD"]
    target_origin_id = aws_s3_bucket.bench_web.bucket

    forwarded_values {
      query_string = true
      cookies {
        forward = "all"
      }
    }

    min_ttl                = 0
    default_ttl            = 3600
    max_ttl                = 86400
    compress               = true
    viewer_protocol_policy = "redirect-to-https"
  }

  custom_error_response {
    error_code         = 404
    response_code      = 200
    response_page_path = "/index.html"
  }

  price_class = "PriceClass_200"

  viewer_certificate {
    acm_certificate_arn      = aws_acm_certificate.justbench_com.arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2019"
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  depends_on = [aws_acm_certificate.justbench_com]

  tags = {
    Name = "bench-${var.env}-global-cloudfront"
  }
}


# 
# Cloudflare DNS Records
# 


resource "cloudflare_record" "root" {
  zone_id = data.cloudflare_zone.justbench_com.id
  name    = "justbench.com"
  type    = "CNAME"
  value   = aws_cloudfront_distribution.bench_web.domain_name
  ttl     = 300
  proxied = false
}

resource "cloudflare_record" "www" {
  zone_id = data.cloudflare_zone.justbench_com.id
  name    = "www.justbench.com"
  type    = "CNAME"
  value   = aws_cloudfront_distribution.bench_web.domain_name
  ttl     = 300
  proxied = false
}

#
# Outputs
#

output "bench_web_certificate_arn" {
  value = aws_acm_certificate.justbench_com.arn
}

output "bench_web_distribution_id" {
  value = aws_cloudfront_distribution.bench_web.id
}

output "bench_web_url" {
  value = aws_cloudfront_distribution.bench_web.domain_name
}
