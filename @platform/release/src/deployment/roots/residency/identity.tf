# let the machine Worker decrypt and send mail
data "aws_caller_identity" "current" {}

data "aws_iam_policy_document" "worker" {
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
  name   = "destack-${var.environment}-machine-${var.residency}"
  policy = data.aws_iam_policy_document.worker.json
}
