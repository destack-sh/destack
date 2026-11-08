locals {
  mail_domain      = "destack.app"
  mail_from_domain = "mail.destack.app"
  report_domain    = split("@", var.dmarc_report_address)[1]
}

# verify destack.app for sending with Easy DKIM
resource "aws_sesv2_email_identity" "destack" {
  email_identity         = local.mail_domain
  configuration_set_name = aws_sesv2_configuration_set.destack.configuration_set_name
}

resource "cloudflare_dns_record" "dkim" {
  count   = 3
  zone_id = cloudflare_zone.domains[local.mail_domain].id
  name    = "${aws_sesv2_email_identity.destack.dkim_signing_attributes[0].tokens[count.index]}._domainkey.${local.mail_domain}"
  type    = "CNAME"
  content = "${aws_sesv2_email_identity.destack.dkim_signing_attributes[0].tokens[count.index]}.dkim.amazonses.com"
  proxied = false
  ttl     = 3600
}

# return bounces to mail.destack.app, which SPF authorizes for SES
resource "aws_sesv2_email_identity_mail_from_attributes" "destack" {
  email_identity         = aws_sesv2_email_identity.destack.email_identity
  mail_from_domain       = local.mail_from_domain
  behavior_on_mx_failure = "USE_DEFAULT_VALUE"
}

resource "cloudflare_dns_record" "mail_from" {
  for_each = {
    mx  = { type = "MX", priority = 10, content = "feedback-smtp.${var.aws_region}.amazonses.com" }
    spf = { type = "TXT", priority = null, content = "\"v=spf1 include:amazonses.com -all\"" }
  }
  zone_id  = cloudflare_zone.domains[local.mail_domain].id
  name     = local.mail_from_domain
  type     = each.value.type
  priority = each.value.priority
  content  = each.value.content
  ttl      = 3600
}

resource "cloudflare_dns_record" "dmarc" {
  zone_id = cloudflare_zone.domains[local.mail_domain].id
  name    = "_dmarc.${local.mail_domain}"
  type    = "TXT"
  content = "\"v=DMARC1; p=quarantine; rua=mailto:${var.dmarc_report_address}\""
  ttl     = 3600
}

# authorize reports for destack.app at a report address on another managed domain
resource "cloudflare_dns_record" "dmarc_report" {
  count   = contains(local.domains, local.report_domain) && local.report_domain != local.mail_domain ? 1 : 0
  zone_id = cloudflare_zone.domains[local.report_domain].id
  name    = "${local.mail_domain}._report._dmarc.${local.report_domain}"
  type    = "TXT"
  content = "\"v=DMARC1\""
  ttl     = 3600
}

# configure TLS, reputation metrics and suppression for sent mail
resource "aws_sesv2_configuration_set" "destack" {
  configuration_set_name = "destack-mail"

  delivery_options {
    tls_policy = "REQUIRE"
  }

  reputation_options {
    reputation_metrics_enabled = true
  }

  sending_options {
    sending_enabled = true
  }

  suppression_options {
    suppressed_reasons = ["BOUNCE", "COMPLAINT"]
  }
}

resource "aws_sesv2_account_suppression_attributes" "destack" {
  suppressed_reasons = ["BOUNCE", "COMPLAINT"]
}

# send only as destack.app under its configuration set
resource "aws_iam_user" "mail" {
  name                 = "destack-mail-send"
  permissions_boundary = "arn:aws:iam::903685415634:policy/destack-mail-send-boundary"
}

data "aws_iam_policy_document" "mail" {
  statement {
    actions = ["ses:SendEmail", "ses:SendRawEmail"]
    resources = [
      aws_sesv2_email_identity.destack.arn,
      aws_sesv2_configuration_set.destack.arn,
    ]
  }
}

resource "aws_iam_user_policy" "mail" {
  name   = "destack-mail-send"
  user   = aws_iam_user.mail.name
  policy = data.aws_iam_policy_document.mail.json
}

resource "aws_iam_access_key" "mail" {
  user = aws_iam_user.mail.name
}
