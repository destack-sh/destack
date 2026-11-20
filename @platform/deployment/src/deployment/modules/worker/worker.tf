# read the platform account
data "aws_caller_identity" "current" {}

# give the Worker one bounded IAM user
resource "aws_iam_user" "worker" {
  name                 = var.name
  permissions_boundary = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:policy/destack-worker-boundary"
}

resource "aws_iam_user_policy" "worker" {
  name   = var.name
  user   = aws_iam_user.worker.name
  policy = var.policy
}

resource "aws_iam_access_key" "worker" {
  user = aws_iam_user.worker.name
}
