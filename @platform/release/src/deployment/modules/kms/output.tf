output "key_arn" {
  description = "The ARN of the key sealing the Worker's keys."
  value       = aws_kms_key.root.arn
}
output "region" {
  description = "The region of the key."
  value       = data.aws_region.current.region
}
output "access_key_id" {
  description = "The Worker's access key id, allowed to decrypt its own keys alone."
  value       = aws_iam_access_key.worker.id
}
output "secret_access_key" {
  description = "The Worker's secret access key."
  value       = aws_iam_access_key.worker.secret
  sensitive   = true
}
