# find the organization's Identity Center and its people
data "aws_ssoadmin_instances" "organization" {
  provider = aws.management
}

locals {
  instance = tolist(data.aws_ssoadmin_instances.organization.arns)[0]
  store    = tolist(data.aws_ssoadmin_instances.organization.identity_store_ids)[0]
}

data "aws_identitystore_user" "people" {
  provider          = aws.management
  for_each          = var.people
  identity_store_id = local.store

  alternate_identifier {
    unique_attribute {
      attribute_path  = "UserName"
      attribute_value = each.key
    }
  }
}

# plan as a person: read everything, and use the state key, as the plan workflow does
resource "aws_ssoadmin_permission_set" "plan" {
  provider         = aws.management
  instance_arn     = local.instance
  name             = "destack-platform-plan"
  session_duration = "PT1H"
}

resource "aws_ssoadmin_managed_policy_attachment" "plan" {
  provider           = aws.management
  instance_arn       = local.instance
  permission_set_arn = aws_ssoadmin_permission_set.plan.arn
  managed_policy_arn = local.read_only
}

resource "aws_ssoadmin_permission_set_inline_policy" "plan" {
  provider           = aws.management
  instance_arn       = local.instance
  permission_set_arn = aws_ssoadmin_permission_set.plan.arn
  inline_policy      = local.state
}

# operate as a person: deploy, root keys and Worker users
resource "aws_ssoadmin_permission_set" "operator" {
  provider         = aws.management
  instance_arn     = local.instance
  name             = "destack-platform-operator"
  session_duration = "PT8H"
}

resource "aws_ssoadmin_managed_policy_attachment" "operator" {
  provider           = aws.management
  instance_arn       = local.instance
  permission_set_arn = aws_ssoadmin_permission_set.operator.arn
  managed_policy_arn = local.power_user
}

resource "aws_ssoadmin_permission_set_inline_policy" "operator" {
  provider           = aws.management
  instance_arn       = local.instance
  permission_set_arn = aws_ssoadmin_permission_set.operator.arn
  inline_policy      = local.operator
}

# administer the accounts themselves
resource "aws_ssoadmin_permission_set" "administrator" {
  provider         = aws.management
  instance_arn     = local.instance
  name             = "AdministratorAccess"
  session_duration = "PT8H"
}

resource "aws_ssoadmin_managed_policy_attachment" "administrator" {
  provider           = aws.management
  instance_arn       = local.instance
  permission_set_arn = aws_ssoadmin_permission_set.administrator.arn
  managed_policy_arn = "arn:aws:iam::aws:policy/AdministratorAccess"
}

# give each person every role on the platform account, and administration of the management account
locals {
  assignments = merge(
    {
      for pair in setproduct(var.people, ["plan", "operator", "administrator"]) :
      "${pair[0]}/platform/${pair[1]}" => { person = pair[0], account = var.platform_account_id, set = pair[1] }
    },
    {
      for person in var.people :
      "${person}/management/administrator" => { person = person, account = var.management_account_id, set = "administrator" }
    },
  )
  permission_sets = {
    plan          = aws_ssoadmin_permission_set.plan.arn
    operator      = aws_ssoadmin_permission_set.operator.arn
    administrator = aws_ssoadmin_permission_set.administrator.arn
  }
}

resource "aws_ssoadmin_account_assignment" "people" {
  provider           = aws.management
  for_each           = local.assignments
  instance_arn       = local.instance
  permission_set_arn = local.permission_sets[each.value.set]
  principal_type     = "USER"
  principal_id       = data.aws_identitystore_user.people[each.value.person].user_id
  target_type        = "AWS_ACCOUNT"
  target_id          = each.value.account
}
