locals {
  # keep two weeks of point-in-time recovery
  backup_days = 14

  # grow storage from 20 GiB, the smallest gp3 volume, to 100 GiB
  storage_gib     = 20
  max_storage_gib = 100

  # sign the server's certificate by the region's RSA 2048 root, which every client verifies
  certificate_authority = "rds-ca-rsa2048-g1"
}

# read the region the database runs in, which picks its authority's root
data "aws_region" "current" {}

# generate the owner role's password
resource "random_password" "role" {
  length  = 40
  special = false
}

# run PostgreSQL encrypted and deletion-protected
resource "aws_db_instance" "database" {
  identifier                   = var.name
  engine                       = "postgres"
  engine_version               = var.major_version
  instance_class               = var.instance_class
  multi_az                     = var.multi_az
  storage_type                 = "gp3"
  allocated_storage            = local.storage_gib
  max_allocated_storage        = local.max_storage_gib
  storage_encrypted            = true
  ca_cert_identifier           = local.certificate_authority
  db_name                      = "postgres"
  username                     = var.role
  password                     = random_password.role.result
  db_subnet_group_name         = aws_db_subnet_group.database.name
  vpc_security_group_ids       = [aws_security_group.database.id]
  publicly_accessible          = true
  backup_retention_period      = local.backup_days
  copy_tags_to_snapshot        = true
  auto_minor_version_upgrade   = true
  deletion_protection          = true
  skip_final_snapshot          = false
  final_snapshot_identifier    = "${var.name}-final"
  performance_insights_enabled = true
  lifecycle {
    prevent_destroy = true
  }
}
