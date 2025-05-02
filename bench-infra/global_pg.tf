#
# Global RDS Aurora 
# NOTE :Infra: global DB should probably be a global Aurora DB, but that requires an expensive memory-optimized instance
# 

resource "random_password" "global_pg_password" {
  length  = 16
  special = false
}

resource "aws_security_group" "global_pg_security_group" {
  name        = "bench-${var.env}-global-pg-security-group"
  description = "Allow inbound traffic to the global RDS cluster"
  vpc_id      = aws_vpc.global_vpc.id

  ingress {
    description = "Allow inbound traffic to the global RDS cluster"
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"] # NOTE :Infra :Security: restrict global PG ingress to VPC/IP ranges
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_db_subnet_group" "global_pg_subnet_group" {
  name       = "bench-${var.env}-global-pg-subnet-group"
  subnet_ids = aws_subnet.global_public[*].id

  tags = {
    Name = "bench-${var.env}-global-pg-subnet-group"
  }
}

resource "aws_rds_cluster" "global_pg_primary" {
  cluster_identifier        = "bench-${var.env}-global-pg-primary"
  engine                    = "aurora-postgresql"
  engine_version            = "16.2"
  database_name             = var.global_pg_name
  master_username           = var.global_pg_username
  master_password           = random_password.global_pg_password.result
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
  identifier                 = "bench-${var.env}-global-pg-${count.index}"
  cluster_identifier         = aws_rds_cluster.global_pg_primary.id
  instance_class             = "db.t3.medium"
  engine                     = aws_rds_cluster.global_pg_primary.engine
  engine_version             = aws_rds_cluster.global_pg_primary.engine_version
  auto_minor_version_upgrade = true
  availability_zone          = local.aws_region_availability_zones[local.global_region][count.index % length(local.aws_region_availability_zones[local.global_region])]
  publicly_accessible        = true
}

output "global_pg_host" {
  value = aws_rds_cluster.global_pg_primary.endpoint
}

output "global_pg_url" {
  value     = "postgresql://${var.global_pg_username}:${random_password.global_pg_password.result}@${aws_rds_cluster.global_pg_primary.endpoint}/${var.global_pg_name}"
  sensitive = true
}
