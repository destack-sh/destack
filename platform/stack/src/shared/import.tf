import {
  for_each = {
    "destack.app"       = "686c311fea501b2ab5625f903bbbbfc1"
    "destack.blog"      = "d2126c9d4dda015d5936724a36e996ea"
    "destack.cloud"     = "58ecb4106a35a2721c466bd636629f12"
    "destack.computer"  = "54812e60fbdf52e5e102ef4a77c72bc2"
    "destack.design"    = "55458691c620095a3f41b2149eee4a95"
    "destack.dev"       = "60f2785b43acf596ecdf9d56540d12f7"
    "destack.me"        = "60ae8b43998b95311d5c98d25d12d166"
    "destack.org"       = "5963075d2ece3509d2c27284d627f7d1"
    "destack.sh"        = "c294d5aa316a6f11a90b4f9ccbdfcad2"
    "destack.site"      = "245f489707d61abb530c87c6832a9098"
    "destack.software"  = "0be8e1f9ac33c51717001be9b8f95685"
    "destack.space"     = "7388799c233b92ff7e57fb5c760dd21e"
    "destack.studio"    = "51436dfbd876db6e208b9fbd0ec61b22"
    "destack.tech"      = "82d23228972334b526961ae3bc9e2263"
    "symbol.industries" = "643d8f0f0ea084d0c31c40098d7faec6"
    "symbolx.com"       = "bfbf7510d962147d9aef13516103f9bf"
  }
  to = cloudflare_zone.domains[each.key]
  id = each.value
}

import {
  for_each = {
    "destack.app"           = "686c311fea501b2ab5625f903bbbbfc1/8980011df106d0509a4bca56b2472c09"
    "www.destack.app"       = "686c311fea501b2ab5625f903bbbbfc1/adc5a09e3723a843b829ba26a291e207"
    "destack.blog"          = "d2126c9d4dda015d5936724a36e996ea/b4cf23ca5d6fb4eac6bfd3683f8c7eed"
    "www.destack.blog"      = "d2126c9d4dda015d5936724a36e996ea/7aea05c3d77ae955b39d7c437ffc9658"
    "destack.cloud"         = "58ecb4106a35a2721c466bd636629f12/66abf53a0c3d7036f7b037fc09cde6bf"
    "www.destack.cloud"     = "58ecb4106a35a2721c466bd636629f12/5c0905513c5c1e068903cccf55349087"
    "destack.computer"      = "54812e60fbdf52e5e102ef4a77c72bc2/8323d09cd4654e8d4e350f737e22f3e2"
    "www.destack.computer"  = "54812e60fbdf52e5e102ef4a77c72bc2/9fa1f45af3fcea060e57ed6d9d8cf031"
    "destack.design"        = "55458691c620095a3f41b2149eee4a95/8cc5478d9e995bb626fdad0f65f2e3a3"
    "www.destack.design"    = "55458691c620095a3f41b2149eee4a95/ef1cb3f8ad71ed2408e815e635779106"
    "destack.dev"           = "60f2785b43acf596ecdf9d56540d12f7/d49cc9a4acf776b5007793c9199104cb"
    "www.destack.dev"       = "60f2785b43acf596ecdf9d56540d12f7/89f272dfc0aa1fb55d137181516124d3"
    "destack.me"            = "60ae8b43998b95311d5c98d25d12d166/330177f7f3e865b5e4c3220f016aa28f"
    "www.destack.me"        = "60ae8b43998b95311d5c98d25d12d166/6ce2010a4cdb341a8d5b605f2d88788d"
    "destack.org"           = "5963075d2ece3509d2c27284d627f7d1/30b8385b7911a9df8afd98fcc8c0d33d"
    "www.destack.org"       = "5963075d2ece3509d2c27284d627f7d1/18e6216137b6aed5ad82bd442044ef13"
    "www.destack.sh"        = "c294d5aa316a6f11a90b4f9ccbdfcad2/59c2f43603b857822ad33143becda2b9"
    "destack.site"          = "245f489707d61abb530c87c6832a9098/1b0e7691d922da2da1bf2bc8dea02974"
    "www.destack.site"      = "245f489707d61abb530c87c6832a9098/781a443ede3ee676791a61f64caf514b"
    "destack.software"      = "0be8e1f9ac33c51717001be9b8f95685/60721b858dd6b0a7ace98a7fcdcdc3ef"
    "www.destack.software"  = "0be8e1f9ac33c51717001be9b8f95685/832353db8726312f56d000a014d3a9ef"
    "destack.space"         = "7388799c233b92ff7e57fb5c760dd21e/7d6229a556f1e9e09008dde8f68c0acf"
    "www.destack.space"     = "7388799c233b92ff7e57fb5c760dd21e/271c8affd844a375ecc9f33242a88dbc"
    "destack.studio"        = "51436dfbd876db6e208b9fbd0ec61b22/cc9a99d550fb37bda15efe08fa07014b"
    "www.destack.studio"    = "51436dfbd876db6e208b9fbd0ec61b22/2df5a8e0526cd6c5ea0509933b438466"
    "destack.tech"          = "82d23228972334b526961ae3bc9e2263/dc0de58235fa9013108543ef5c7c4305"
    "www.destack.tech"      = "82d23228972334b526961ae3bc9e2263/7f21f820f9446eb068305f1ecfdb86cc"
    "symbol.industries"     = "643d8f0f0ea084d0c31c40098d7faec6/a4e33c4686147d6a9e58c4dcda7267e8"
    "www.symbol.industries" = "643d8f0f0ea084d0c31c40098d7faec6/28ae49274e5893d824163f1ccb2acbf9"
  }
  to = cloudflare_dns_record.redirects[each.key]
  id = each.value
}

import {
  for_each = {
    "destack.app"       = "zones/686c311fea501b2ab5625f903bbbbfc1/e3a2de1287754be28d5e5173b716d128"
    "destack.blog"      = "zones/d2126c9d4dda015d5936724a36e996ea/f8bbf09a894f4789bb4b5c094c03a59d"
    "destack.cloud"     = "zones/58ecb4106a35a2721c466bd636629f12/801de1d96121483bb77e40cb030d749c"
    "destack.computer"  = "zones/54812e60fbdf52e5e102ef4a77c72bc2/532377440a42407a86f570bf3983ac40"
    "destack.design"    = "zones/55458691c620095a3f41b2149eee4a95/f3294c21405e4bcdb0791e6d74f48a8f"
    "destack.dev"       = "zones/60f2785b43acf596ecdf9d56540d12f7/f3f6424417ff44358e02d24ea6640317"
    "destack.me"        = "zones/60ae8b43998b95311d5c98d25d12d166/33035920bf7440a19f183776880ed014"
    "destack.org"       = "zones/5963075d2ece3509d2c27284d627f7d1/15ddc39bbd3546c6bafb94254dc4ee57"
    "destack.sh"        = "zones/c294d5aa316a6f11a90b4f9ccbdfcad2/a316eeba33a84707bd2963697f6f0641"
    "destack.site"      = "zones/245f489707d61abb530c87c6832a9098/feced674c99f4d21aad82473330714df"
    "destack.software"  = "zones/0be8e1f9ac33c51717001be9b8f95685/36e60d5bb364441991cdaeee44828fbb"
    "destack.space"     = "zones/7388799c233b92ff7e57fb5c760dd21e/86cad96f25f74f9abc1d7dc171e53d0b"
    "destack.studio"    = "zones/51436dfbd876db6e208b9fbd0ec61b22/f6f8c443044f4620b9ff8763e3c82ae1"
    "destack.tech"      = "zones/82d23228972334b526961ae3bc9e2263/e3617f28ed7641ab9073e4434ff37ec9"
    "symbol.industries" = "zones/643d8f0f0ea084d0c31c40098d7faec6/29b48b6c0f55406d8c7908a4e4eae1e9"
  }
  to = cloudflare_ruleset.redirects[each.key]
  id = each.value
}

import {
  to = cloudflare_r2_bucket.infrastructure
  id = "27c0d00fb3a27a4ccbf46a3cceab9301/destack-infrastructure/default"
}

import {
  to = cloudflare_dns_record.site
  id = "c294d5aa316a6f11a90b4f9ccbdfcad2/5c6afa58c3dbd04f2e4a6e1678b9577d"
}

import {
  to = cloudflare_workers_route.site
  id = "c294d5aa316a6f11a90b4f9ccbdfcad2/9586915abce0479a85c907c0dcb62f60"
}
