variable "name" {
  description = "The Worker's name, such as destack-production-universe."
  type        = string
}

variable "policy" {
  description = "What the Worker may do, as an IAM policy document, within the Worker boundary."
  type        = string
}
