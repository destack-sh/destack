# answer nothing until a deploy ships the Worker's code
locals {
  first_module = "export default { fetch: () => new Response(null, { status: 503 }), tail: () => {} };"
}

# create the machine's and its tail's Workers, whose code each deploy ships, so every binding between them resolves
resource "cloudflare_worker" "machine" {
  account_id = var.account_id
  name       = "destack-${var.environment}-machine-${var.residency}"

  # keep the Worker, whose deletion deletes its objects, and the settings each deploy writes
  lifecycle {
    prevent_destroy = true
    ignore_changes  = [logpush, observability, subdomain, tags, tail_consumers]
  }
}

resource "cloudflare_worker" "tail" {
  account_id = var.account_id
  name       = "destack-${var.environment}-tail-${var.residency}"

  # keep the settings each deploy writes
  lifecycle {
    ignore_changes = [logpush, observability, subdomain, tags, tail_consumers]
  }
}

# deploy each Worker once at its creation, as a binding's target needs a deployment, leaving every later one to the deploys
resource "cloudflare_worker_version" "first" {
  for_each    = { machine = cloudflare_worker.machine.id, tail = cloudflare_worker.tail.id }
  account_id  = var.account_id
  worker_id   = each.value
  main_module = "first.mjs"
  modules = [{
    name           = "first.mjs"
    content_type   = "application/javascript+module"
    content_base64 = base64encode(local.first_module)
  }]

  lifecycle {
    ignore_changes = all
  }
}

resource "cloudflare_workers_deployment" "first" {
  for_each    = { machine = cloudflare_worker.machine.name, tail = cloudflare_worker.tail.name }
  account_id  = var.account_id
  script_name = each.value
  strategy    = "percentage"
  versions    = [{ version_id = cloudflare_worker_version.first[each.key].id, percentage = 100 }]

  lifecycle {
    ignore_changes = all
  }
}
