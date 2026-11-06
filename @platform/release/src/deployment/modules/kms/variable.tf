variable "name" {
  description = "The name of the key and of the Worker's IAM user, such as destack-production-universe."
  type        = string
}
variable "operators" {
  description = "The roles that create, repolicy, grant and replicate the key, as ARN patterns."
  type        = list(string)
}
variable "owner" {
  description = "The encryption context owner the Worker decrypts under, such as universe or machine <id>."
  type        = string
}
