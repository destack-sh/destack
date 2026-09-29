# transfer package buckets to regional state without deleting their contents
removed {
  from = cloudflare_r2_bucket.packages
  lifecycle {
    destroy = false
  }
}
provider "cloudflare" {
  alias = "workers"
}
