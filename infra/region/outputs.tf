output "vpc_id" {
  value = aws_vpc.region_vpc.id
}

output "vpc_cidr_block" {
  value = aws_vpc.region_vpc.cidr_block
}

output "public_subnets" {
  value = aws_subnet.public[*].id
}

output "private_subnets" {
  value = aws_subnet.private[*].id
}

output "region_cluster" {
  value = module.cluster_0
}
