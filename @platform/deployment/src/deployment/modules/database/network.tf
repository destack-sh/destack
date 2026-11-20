locals {
  # the database's network, one /20 per zone
  network = "10.0.0.0/16"
}

# place the database in its own network
data "aws_availability_zones" "available" {
  state = "available"
}

resource "aws_vpc" "database" {
  cidr_block           = local.network
  enable_dns_support   = true
  enable_dns_hostnames = true
  tags                 = { Name = var.name }
}

resource "aws_internet_gateway" "database" {
  vpc_id = aws_vpc.database.id
  tags   = { Name = var.name }
}

resource "aws_route_table" "database" {
  vpc_id = aws_vpc.database.id
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.database.id
  }
  tags = { Name = var.name }
}

# spread the subnets across the region's zones
resource "aws_subnet" "database" {
  for_each          = toset(data.aws_availability_zones.available.names)
  vpc_id            = aws_vpc.database.id
  availability_zone = each.key
  cidr_block        = cidrsubnet(local.network, 4, index(data.aws_availability_zones.available.names, each.key))
  tags              = { Name = "${var.name}-${each.key}" }
}

resource "aws_route_table_association" "database" {
  for_each       = aws_subnet.database
  subnet_id      = each.value.id
  route_table_id = aws_route_table.database.id
}

resource "aws_db_subnet_group" "database" {
  name       = var.name
  subnet_ids = [for subnet in aws_subnet.database : subnet.id]
}

# accept PostgreSQL from the clients
resource "aws_security_group" "database" {
  name   = var.name
  vpc_id = aws_vpc.database.id
}

resource "aws_vpc_security_group_ingress_rule" "clients" {
  for_each          = toset(var.clients)
  security_group_id = aws_security_group.database.id
  cidr_ipv4         = each.key
  ip_protocol       = "tcp"
  from_port         = 5432
  to_port           = 5432
}
