output "arn" {
  description = "The ARN of the Worker's user, which resource policies grant."
  value       = aws_iam_user.worker.arn
}

output "access_key_id" {
  description = "The Worker's access key id."
  value       = aws_iam_access_key.worker.id
}

output "secret_access_key" {
  description = "The Worker's secret access key."
  value       = aws_iam_access_key.worker.secret
  sensitive   = true
}
