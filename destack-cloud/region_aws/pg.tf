#
# Regional RDS Aurora
#

resource "random_password" "regional_pg_password" {
  length  = 32
  special = false
}

resource "aws_security_group" "regional_pg_security_group" {
  name        = "destack-${var.env}-${var.region}-regional-pg-security-group"
  description = "Allow inbound traffic to the regional RDS cluster"
  vpc_id      = aws_vpc.region_vpc.id

  ingress {
    description = "Allow inbound traffic to the regional RDS cluster"
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"] # NOTE :Infra :Security: restrict regional PG ingress to VPC/IP ranges
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_db_subnet_group" "regional_pg_subnet_group" {
  name       = "destack-${var.env}-${var.region}-regional-pg-subnet-group"
  subnet_ids = aws_subnet.public[*].id

  tags = {
    Name = "destack-${var.env}-${var.region}-regional-pg-subnet-group"
  }
}

resource "aws_rds_cluster" "regional_pg_primary" {
  cluster_identifier        = "destack-${var.env}-${var.region}-regional-pg-primary"
  engine                    = "aurora-postgresql"
  engine_version            = "16.2"
  database_name             = var.regional_pg_name
  master_username           = var.regional_pg_username
  master_password           = random_password.regional_pg_password.result
  backup_retention_period   = 7
  preferred_backup_window   = "06:00-08:00"
  storage_encrypted         = true
  final_snapshot_identifier = "destack-${var.env}-${var.region}-regional-pg-final-snapshot"
  skip_final_snapshot       = true

  vpc_security_group_ids = [aws_security_group.regional_pg_security_group.id]
  db_subnet_group_name   = aws_db_subnet_group.regional_pg_subnet_group.name
}

resource "aws_rds_cluster_instance" "regional_pg_primary_instance" {
  count                      = 1
  identifier                 = "destack-${var.env}-${var.region}-regional-pg-${count.index}"
  cluster_identifier         = aws_rds_cluster.regional_pg_primary.id
  instance_class             = "db.t3.medium"
  engine                     = aws_rds_cluster.regional_pg_primary.engine
  engine_version             = aws_rds_cluster.regional_pg_primary.engine_version
  auto_minor_version_upgrade = true
  availability_zone          = var.aws_availability_zones[count.index % length(var.aws_availability_zones)]
  publicly_accessible        = true
}

output "regional_pg_host" {
  value = aws_rds_cluster.regional_pg_primary.endpoint
}

output "regional_pg_url" {
  value     = "postgresql://${var.regional_pg_username}:${random_password.regional_pg_password.result}@${aws_rds_cluster.regional_pg_primary.endpoint}/${var.regional_pg_name}"
  sensitive = true
}
