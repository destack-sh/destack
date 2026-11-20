output "role" {
  description = "The role its service connects as: its host, user and password."
  sensitive   = true
  value = {
    host     = aws_db_instance.database.address
    user     = aws_db_instance.database.username
    password = random_password.role.result
  }
}

output "authority" {
  description = "The root certificate of the region's RDS authority the database is pinned to, which clients verify it against."
  value       = file("${path.module}/authority/${data.aws_region.current.region}.pem")
}
