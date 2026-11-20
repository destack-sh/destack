variable "name" {
  description = "The database's name, such as destack-production-universe."
  type        = string
}

variable "instance_class" {
  description = "The instance class, such as db.t4g.medium."
  type        = string
}

variable "multi_az" {
  description = "Whether a standby in a second zone takes over when the instance's zone fails."
  type        = bool
}

variable "major_version" {
  description = "The PostgreSQL major version, such as 17."
  type        = string
}

variable "role" {
  description = "The one role its service connects and migrates as, the instance's owner."
  type        = string
}

variable "clients" {
  description = "The IPv4 ranges that may connect: Cloudflare's, where Hyperdrive and the release's deploys reach it from."
  type        = list(string)
}
