

#
# Let's Encrypt certificate
# 

provider "acme" {
  server_url = "https://acme-v02.api.letsencrypt.org/directory"
}

resource "acme_registration" "reg" {
  email_address = "florian@symbolx.com"
}

resource "acme_certificate" "main_website" {
  account_key_pem = acme_registration.reg.account_key_pem
  common_name     = local.main_website
  subject_alternative_names = [
    "*.${local.main_website}",
    "*.host.${local.main_website}",
    "*.destack.${local.main_website}"
  ]

  dns_challenge {
    provider = "cloudflare"

    config = {
      CLOUDFLARE_DNS_API_TOKEN = var.cloudflare_api_token
    }
  }

  lifecycle {
    create_before_destroy = true
  }
}

# 
# ACM certificate (derived from LE cert for within AWS)
#

provider "aws" {
  alias  = "us-east-1"
  region = "us-east-1"
}
resource "aws_acm_certificate" "main_website" {
  provider = aws.us-east-1 // all ACM certificates must be in us-east-1

  certificate_body  = acme_certificate.main_website.certificate_pem
  private_key       = acme_certificate.main_website.private_key_pem
  certificate_chain = acme_certificate.main_website.issuer_pem

  tags = {
    Name = "destack-${var.env}-global-web-cert"
  }
}
