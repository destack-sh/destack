# adopt the existing EU package buckets after releasing their shared state entries
import {
  for_each = var.package_import_id == null ? toset([]) : toset([var.package_import_id])
  to       = cloudflare_r2_bucket.packages
  id       = each.value
}
