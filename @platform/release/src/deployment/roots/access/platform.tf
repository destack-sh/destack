locals {
  # GitHub's token issuer, which the workflows' roles trust
  github = "token.actions.githubusercontent.com"

  # read everything in the platform account, which holds the platform alone
  read_only = "arn:aws:iam::aws:policy/ReadOnlyAccess"

  # render each persona's policy for the platform account, alike for roles and permission sets
  state  = templatefile("${path.module}/policy/state.json", { account = var.platform_account_id })
  deploy = templatefile("${path.module}/policy/deploy.json", { account = var.platform_account_id })

  # merge the operator's policy with the state's, as a permission set takes one inline policy
  operator = jsonencode({
    Version = "2012-10-17"
    Statement = concat(
      jsondecode(templatefile("${path.module}/policy/operator.json", { account = var.platform_account_id })).Statement,
      jsondecode(local.state).Statement,
    )
  })
}

# let GitHub's workflows assume the platform's roles with short-lived tokens, never keys
resource "aws_iam_openid_connect_provider" "github" {
  url             = "https://${local.github}"
  client_id_list  = ["sts.amazonaws.com"]
  thumbprint_list = ["ab9d0263244dd0326eb67015705a667e79cfe998"]
}

# trust each workflow environment's token for its role alone
data "aws_iam_policy_document" "workflow" {
  for_each = toset(["plan", "production"])

  statement {
    actions = ["sts:AssumeRoleWithWebIdentity"]
    principals {
      type        = "Federated"
      identifiers = [aws_iam_openid_connect_provider.github.arn]
    }
    condition {
      test     = "StringEquals"
      variable = "${local.github}:aud"
      values   = ["sts.amazonaws.com"]
    }
    condition {
      test     = "StringEquals"
      variable = "${local.github}:sub"
      values   = ["repo:${var.repository}:environment:${each.key}"]
    }
  }
}

# plan from the plan environment: read everything, and use the state key
resource "aws_iam_role" "plan" {
  name                 = "destack-platform-plan"
  assume_role_policy   = data.aws_iam_policy_document.workflow["plan"].json
  max_session_duration = 3600
}

resource "aws_iam_role_policy" "plan_state" {
  role   = aws_iam_role.plan.name
  name   = "destack-platform-state"
  policy = local.state
}

resource "aws_iam_role_policies_exclusive" "plan" {
  role_name    = aws_iam_role.plan.name
  policy_names = [aws_iam_role_policy.plan_state.name]
}

resource "aws_iam_role_policy_attachments_exclusive" "plan" {
  role_name   = aws_iam_role.plan.name
  policy_arns = [local.read_only]
}

# deploy from the production environment, never creating root keys or the Workers' users
resource "aws_iam_role" "deploy" {
  name                 = "destack-platform-deploy"
  assume_role_policy   = data.aws_iam_policy_document.workflow["production"].json
  max_session_duration = 3600
}

resource "aws_iam_role_policy" "deploy" {
  role   = aws_iam_role.deploy.name
  name   = "destack-platform-deploy"
  policy = local.deploy
}

resource "aws_iam_role_policy" "deploy_state" {
  role   = aws_iam_role.deploy.name
  name   = "destack-platform-state"
  policy = local.state
}

resource "aws_iam_role_policies_exclusive" "deploy" {
  role_name    = aws_iam_role.deploy.name
  policy_names = [aws_iam_role_policy.deploy.name, aws_iam_role_policy.deploy_state.name]
}

resource "aws_iam_role_policy_attachments_exclusive" "deploy" {
  role_name   = aws_iam_role.deploy.name
  policy_arns = []
}

# bound the Workers' users: decrypting their own root keys, and sending the platform's mail
resource "aws_iam_policy" "kms_decrypt_boundary" {
  name        = "destack-kms-decrypt-boundary"
  description = "Maximum permissions for Destack production KMS worker users"
  policy      = templatefile("${path.module}/policy/kms-decrypt-boundary.json", { account = var.platform_account_id })
}

resource "aws_iam_policy" "mail_send_boundary" {
  name   = "destack-mail-send-boundary"
  policy = templatefile("${path.module}/policy/mail-send-boundary.json", { account = var.platform_account_id })
}
