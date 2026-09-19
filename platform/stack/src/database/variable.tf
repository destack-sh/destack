variable "organization" {
  description = "PlanetScale organization."
  type        = string
}
variable "name" {
  description = "Database name."
  type        = string
}
variable "region" {
  description = "PlanetScale region."
  type        = string
}
variable "cluster_size" {
  description = "Postgres cluster size."
  type        = string
}
variable "major_version" {
  description = "Postgres major version."
  type        = string
}
