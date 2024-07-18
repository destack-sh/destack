#
# Global RDS Aurora 
# NOTE :Infra: global DB should probably be a global Aurora DB, but that requires an expensive memory-optimized instance
# 

resource "aws_security_group" "global_pg_security_group" {
  name        = "bench-${var.env}-global-pg-security-group"
  description = "Allow inbound traffic to the global RDS cluster"
  vpc_id      = aws_vpc.global_vpc.id

  ingress {
    description = "Allow inbound traffic to the global RDS cluster"
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/16"]
  }
}

resource "aws_db_subnet_group" "global_pg_subnet_group" {
  name       = "bench-${var.env}-global-pg-subnet-group"
  subnet_ids = aws_subnet.global_private[*].id

  tags = {
    Name = "bench-${var.env}-rds-subnet-group"
  }
}

resource "aws_rds_cluster" "global_pg_primary" {
  cluster_identifier        = "bench-${var.env}-global-pg-primary"
  engine                    = "aurora-postgresql"
  engine_version            = "16.2"
  database_name             = var.global_pg_name
  master_username           = var.global_pg_username
  master_password           = var.global_pg_password
  backup_retention_period   = 7
  preferred_backup_window   = "06:00-08:00"
  storage_encrypted         = true
  final_snapshot_identifier = "bench-${var.env}-global-pg-final-snapshot"
  skip_final_snapshot       = true

  vpc_security_group_ids = [aws_security_group.global_pg_security_group.id]
  db_subnet_group_name   = aws_db_subnet_group.global_pg_subnet_group.name
}

resource "aws_rds_cluster_instance" "global_pg_primary_instance" {
  count                      = 1
  identifier                 = "bench-${var.env}-global-db-${count.index}"
  cluster_identifier         = aws_rds_cluster.global_pg_primary.id
  instance_class             = "db.t3.medium"
  engine                     = aws_rds_cluster.global_pg_primary.engine
  engine_version             = aws_rds_cluster.global_pg_primary.engine_version
  auto_minor_version_upgrade = true
  availability_zone          = var.region_availability_zones[var.global_region][count.index]
}
