
#
# S3 buckets
# 

resource "aws_s3_bucket" "bench_public" {
  bucket = "bench-${var.env}-${var.region}-public"
  tags = {
    Name = "bench-${var.env}-${var.region}-public"
  }
}


# 
# EKS (system) nodes
#

resource "aws_eks_node_group" "region_system_nodes" {
  cluster_name    = aws_eks_cluster.region_cluster.name
  node_group_name = "bench-${var.env}-${var.region}-region-system-nodes"
  node_role_arn   = aws_iam_role.region_node_role.arn
  subnet_ids      = aws_subnet.private[*].id

  scaling_config {
    desired_size = var.system_desired_cluster_size
    max_size     = var.system_max_cluster_size
    min_size     = var.system_min_cluster_size
  }

  instance_types = [var.system_node_instance_type]
}


#
# Envoy
# 

# ...
