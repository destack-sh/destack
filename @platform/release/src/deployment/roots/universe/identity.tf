# let the universe Worker decrypt, keep files and send mail
data "aws_caller_identity" "current" {}

data "aws_iam_policy_document" "worker" {
  statement {
    sid       = "Files"
    actions   = ["s3:GetObject", "s3:PutObject", "s3:DeleteObject", "s3:AbortMultipartUpload", "s3:ListMultipartUploadParts"]
    resources = ["${aws_s3_bucket.files.arn}/*"]
  }
  statement {
    sid       = "FileListing"
    actions   = ["s3:ListBucket", "s3:ListBucketMultipartUploads"]
    resources = [aws_s3_bucket.files.arn]
  }
  statement {
    sid     = "Mail"
    actions = ["ses:SendEmail", "ses:SendRawEmail"]
    resources = [
      "arn:aws:ses:${var.mail_region}:${data.aws_caller_identity.current.account_id}:identity/destack.app",
      "arn:aws:ses:${var.mail_region}:${data.aws_caller_identity.current.account_id}:configuration-set/destack-mail",
    ]
  }
}

module "worker" {
  source = "../../modules/worker"
  name   = "destack-${var.environment}-universe"
  policy = data.aws_iam_policy_document.worker.json
}
