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
  description = "The root certificates of the region's RDS authorities, which clients verify the database against."
  value       = file("${path.module}/authority/${data.aws_region.current.region}.pem")
}
