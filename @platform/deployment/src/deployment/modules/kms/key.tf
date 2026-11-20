# the account administering and sealing under the key, in its region
data "aws_caller_identity" "current" {}

data "aws_region" "current" {}

locals {
  account = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
}

data "aws_iam_policy_document" "key" {
  # let the account administer the key without decrypting under it
  statement {
    sid = "Administer"
    principals {
      type        = "AWS"
      identifiers = [local.account]
    }
    actions = [
      "kms:CancelKeyDeletion",
      "kms:CreateAlias",
      "kms:Delete*",
      "kms:Describe*",
      "kms:Disable*",
      "kms:Enable*",
      "kms:Get*",
      "kms:List*",
      "kms:Put*",
      "kms:ReplicateKey",
      "kms:Revoke*",
      "kms:ScheduleKeyDeletion",
      "kms:TagResource",
      "kms:UntagResource",
      "kms:Update*",
    ]
    resources = ["*"]
  }

  # let the account seal keys it never reads back
  statement {
    sid = "Envelope"
    principals {
      type        = "AWS"
      identifiers = [local.account]
    }
    actions   = ["kms:Encrypt", "kms:GenerateDataKeyWithoutPlaintext"]
    resources = ["*"]
  }

  # let the Worker decrypt only what is sealed for its owner
  statement {
    sid = "Unseal"
    principals {
      type        = "AWS"
      identifiers = [var.worker]
    }
    actions   = ["kms:Decrypt"]
    resources = ["*"]
    condition {
      test     = "StringEquals"
      variable = "kms:EncryptionContext:owner"
      values   = [var.owner]
    }
  }

  # refuse decrypting to anyone but the Worker, whatever a grant or an identity policy allows
  statement {
    sid    = "Confine"
    effect = "Deny"
    principals {
      type        = "AWS"
      identifiers = ["*"]
    }
    actions   = ["kms:Decrypt"]
    resources = ["*"]
    condition {
      test     = "StringNotEquals"
      variable = "aws:PrincipalArn"
      values   = [var.worker]
    }
  }

  # refuse repolicing, regranting, replicating, disabling or deleting the key to anyone but the operators
  statement {
    sid    = "Protect"
    effect = "Deny"
    principals {
      type        = "AWS"
      identifiers = ["*"]
    }
    actions = [
      "kms:CreateGrant",
      "kms:DisableKey",
      "kms:PutKeyPolicy",
      "kms:ReplicateKey",
      "kms:ScheduleKeyDeletion",
      "kms:UpdatePrimaryRegion",
    ]
    resources = ["*"]
    condition {
      test     = "ArnNotLike"
      variable = "aws:PrincipalArn"
      values   = var.operators
    }
  }
}

# seal the Worker's keys under an HSM-backed key
resource "aws_kms_key" "root" {
  description             = "The root keys of ${var.name}."
  key_usage               = "ENCRYPT_DECRYPT"
  multi_region            = true
  enable_key_rotation     = true
  deletion_window_in_days = 30
  policy                  = data.aws_iam_policy_document.key.json
  tags                    = { "destack-root" = var.name }
}

resource "aws_kms_alias" "root" {
  name          = "alias/${var.name}"
  target_key_id = aws_kms_key.root.key_id
}
