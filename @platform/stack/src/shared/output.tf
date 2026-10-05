output "mail_access_key_id" {
  description = "Access key id the universe sends mail through SES with."
  value       = aws_iam_access_key.mail.id
}

output "mail_secret_access_key" {
  description = "Secret access key the universe sends mail through SES with."
  value       = aws_iam_access_key.mail.secret
  sensitive   = true
}
