output "vpc_id" {
  value = aws_vpc.eks_vpc.id
}

output "vpc_cidr_block" {
  value = aws_vpc.eks_vpc.cidr_block
}

output "public_subnets" {
  value = aws_subnet.public[*].id
}

output "private_subnets" {
  value = aws_subnet.private[*].id
}

output "eks_cluster" {
  value = aws_eks_cluster.eks_cluster
}