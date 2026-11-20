output "key_arn" {
  description = "The ARN of the key sealing the Worker's keys."
  value       = aws_kms_key.root.arn
}

output "region" {
  description = "The region of the key."
  value       = data.aws_region.current.region
}

output "policy" {
  description = "The key's policy, which its replicas take too."
  value       = data.aws_iam_policy_document.key.json
}
