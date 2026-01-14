use super::super::source::{BuiltinLib, BuiltinLibSource};

// deno.ns sources
const LIB_DENO_NS_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.ns/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.ns/v2.6/index.d.ts")),
);
const LIB_DENO_NS_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.ns/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.ns/v2.5/index.d.ts")),
);

// deno.net sources
const LIB_DENO_NET_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.net/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.net/v2.6/index.d.ts")),
);
const LIB_DENO_NET_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.net/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.net/v2.5/index.d.ts")),
);

// deno.shared_globals sources
const LIB_DENO_SHARED_GLOBALS_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.shared_globals/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.shared_globals/v2.6/index.d.ts")),
);
const LIB_DENO_SHARED_GLOBALS_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.shared_globals/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.shared_globals/v2.5/index.d.ts")),
);

// deno.unstable sources
const LIB_DENO_UNSTABLE_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.unstable/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.unstable/v2.6/index.d.ts")),
);
const LIB_DENO_UNSTABLE_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.unstable/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.unstable/v2.5/index.d.ts")),
);

// deno.window sources
const LIB_DENO_WINDOW_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.window/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.window/v2.6/index.d.ts")),
);
const LIB_DENO_WINDOW_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.window/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.window/v2.5/index.d.ts")),
);

// deno.worker sources
const LIB_DENO_WORKER_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.worker/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.worker/v2.6/index.d.ts")),
);
const LIB_DENO_WORKER_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.worker/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.worker/v2.5/index.d.ts")),
);

// deno.broadcast_channel sources
const LIB_DENO_BROADCAST_CHANNEL_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.broadcast_channel/v2.6",
    "index.d.ts",
    include_str!(concat!(
        "../../../lib/deno.broadcast_channel/v2.6/index.d.ts"
    )),
);
const LIB_DENO_BROADCAST_CHANNEL_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.broadcast_channel/v2.5",
    "index.d.ts",
    include_str!(concat!(
        "../../../lib/deno.broadcast_channel/v2.5/index.d.ts"
    )),
);

// deno.cache sources
const LIB_DENO_CACHE_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.cache/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.cache/v2.6/index.d.ts")),
);
const LIB_DENO_CACHE_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.cache/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.cache/v2.5/index.d.ts")),
);

// deno.canvas sources
const LIB_DENO_CANVAS_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.canvas/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.canvas/v2.6/index.d.ts")),
);
const LIB_DENO_CANVAS_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.canvas/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.canvas/v2.5/index.d.ts")),
);

// deno.console sources
const LIB_DENO_CONSOLE_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.console/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.console/v2.6/index.d.ts")),
);
const LIB_DENO_CONSOLE_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.console/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.console/v2.5/index.d.ts")),
);

// deno.crypto sources
const LIB_DENO_CRYPTO_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.crypto/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.crypto/v2.6/index.d.ts")),
);
const LIB_DENO_CRYPTO_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.crypto/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.crypto/v2.5/index.d.ts")),
);

// deno.fetch sources
const LIB_DENO_FETCH_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.fetch/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.fetch/v2.6/index.d.ts")),
);
const LIB_DENO_FETCH_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.fetch/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.fetch/v2.5/index.d.ts")),
);

// deno.url sources
const LIB_DENO_URL_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.url/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.url/v2.6/index.d.ts")),
);
const LIB_DENO_URL_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.url/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.url/v2.5/index.d.ts")),
);

// deno.web sources
const LIB_DENO_WEB_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.web/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.web/v2.6/index.d.ts")),
);
const LIB_DENO_WEB_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.web/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.web/v2.5/index.d.ts")),
);

// deno.webgpu sources
const LIB_DENO_WEBGPU_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.webgpu/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.webgpu/v2.6/index.d.ts")),
);
const LIB_DENO_WEBGPU_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.webgpu/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.webgpu/v2.5/index.d.ts")),
);

// deno.websocket sources
const LIB_DENO_WEBSOCKET_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.websocket/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.websocket/v2.6/index.d.ts")),
);
const LIB_DENO_WEBSOCKET_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.websocket/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.websocket/v2.5/index.d.ts")),
);

// deno.webstorage sources
const LIB_DENO_WEBSTORAGE_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.webstorage/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.webstorage/v2.6/index.d.ts")),
);
const LIB_DENO_WEBSTORAGE_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno.webstorage/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno.webstorage/v2.5/index.d.ts")),
);

// deno
pub const LIB_DENO: BuiltinLib =
    BuiltinLib::ambient_lib("deno", &[LIB_DENO_NS_V2_6_INDEX_D_DS], &["deno.window"]);
pub const LIB_DENO_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.v2.6",
    &[LIB_DENO_NS_V2_6_INDEX_D_DS],
    &["deno.window.v2.6"],
);
pub const LIB_DENO_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.v2.5",
    &[LIB_DENO_NS_V2_5_INDEX_D_DS],
    &["deno.window.v2.5"],
);

// deno.ns
pub const LIB_DENO_NS: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.ns",
    &[LIB_DENO_NS_V2_6_INDEX_D_DS],
    &["esnext", "deno.net", "deno.shared_globals"],
);
pub const LIB_DENO_NS_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.ns.v2.6",
    &[LIB_DENO_NS_V2_6_INDEX_D_DS],
    &["esnext", "deno.net.v2.6", "deno.shared_globals.v2.6"],
);
pub const LIB_DENO_NS_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.ns.v2.5",
    &[LIB_DENO_NS_V2_5_INDEX_D_DS],
    &["esnext", "deno.net.v2.5", "deno.shared_globals.v2.5"],
);

// deno.net
pub const LIB_DENO_NET: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.net",
    &[LIB_DENO_NET_V2_6_INDEX_D_DS],
    &["esnext", "esnext.disposable"],
);
pub const LIB_DENO_NET_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.net.v2.6",
    &[LIB_DENO_NET_V2_6_INDEX_D_DS],
    &["esnext", "esnext.disposable"],
);
pub const LIB_DENO_NET_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.net.v2.5",
    &[LIB_DENO_NET_V2_5_INDEX_D_DS],
    &["esnext", "esnext.disposable"],
);

// deno.shared_globals
pub const LIB_DENO_SHARED_GLOBALS: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.shared_globals",
    &[LIB_DENO_SHARED_GLOBALS_V2_6_INDEX_D_DS],
    &[
        "esnext",
        "deno.console",
        "deno.url",
        "deno.web",
        "deno.webgpu",
        "deno.canvas",
        "deno.fetch",
        "deno.websocket",
        "deno.crypto",
        "deno.ns",
        "deno.broadcast_channel",
    ],
);
pub const LIB_DENO_SHARED_GLOBALS_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.shared_globals.v2.6",
    &[LIB_DENO_SHARED_GLOBALS_V2_6_INDEX_D_DS],
    &[
        "esnext",
        "deno.console.v2.6",
        "deno.url.v2.6",
        "deno.web.v2.6",
        "deno.webgpu.v2.6",
        "deno.canvas.v2.6",
        "deno.fetch.v2.6",
        "deno.websocket.v2.6",
        "deno.crypto.v2.6",
        "deno.ns.v2.6",
        "deno.broadcast_channel.v2.6",
    ],
);
pub const LIB_DENO_SHARED_GLOBALS_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.shared_globals.v2.5",
    &[LIB_DENO_SHARED_GLOBALS_V2_5_INDEX_D_DS],
    &[
        "esnext",
        "deno.console.v2.5",
        "deno.url.v2.5",
        "deno.web.v2.5",
        "deno.webgpu.v2.5",
        "deno.canvas.v2.5",
        "deno.fetch.v2.5",
        "deno.websocket.v2.5",
        "deno.crypto.v2.5",
        "deno.ns.v2.5",
        "deno.broadcast_channel.v2.5",
    ],
);

// deno.unstable
pub const LIB_DENO_UNSTABLE: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.unstable",
    &[LIB_DENO_UNSTABLE_V2_6_INDEX_D_DS],
    &["deno.ns", "esnext", "es2022.intl"],
);
pub const LIB_DENO_UNSTABLE_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.unstable.v2.6",
    &[LIB_DENO_UNSTABLE_V2_6_INDEX_D_DS],
    &["deno.ns.v2.6", "esnext", "es2022.intl"],
);
pub const LIB_DENO_UNSTABLE_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.unstable.v2.5",
    &[LIB_DENO_UNSTABLE_V2_5_INDEX_D_DS],
    &["deno.ns.v2.5", "esnext", "es2022.intl"],
);

// deno.window
pub const LIB_DENO_WINDOW: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.window",
    &[LIB_DENO_WINDOW_V2_6_INDEX_D_DS],
    &[
        "deno.ns",
        "deno.shared_globals",
        "deno.webstorage",
        "esnext",
        "deno.cache",
    ],
);
pub const LIB_DENO_WINDOW_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.window.v2.6",
    &[LIB_DENO_WINDOW_V2_6_INDEX_D_DS],
    &[
        "deno.ns.v2.6",
        "deno.shared_globals.v2.6",
        "deno.webstorage.v2.6",
        "esnext",
        "deno.cache.v2.6",
    ],
);
pub const LIB_DENO_WINDOW_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.window.v2.5",
    &[LIB_DENO_WINDOW_V2_5_INDEX_D_DS],
    &[
        "deno.ns.v2.5",
        "deno.shared_globals.v2.5",
        "deno.webstorage.v2.5",
        "esnext",
        "deno.cache.v2.5",
    ],
);

// deno.worker
pub const LIB_DENO_WORKER: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.worker",
    &[LIB_DENO_WORKER_V2_6_INDEX_D_DS],
    &["deno.ns", "deno.shared_globals", "esnext", "deno.cache"],
);
pub const LIB_DENO_WORKER_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.worker.v2.6",
    &[LIB_DENO_WORKER_V2_6_INDEX_D_DS],
    &[
        "deno.ns.v2.6",
        "deno.shared_globals.v2.6",
        "esnext",
        "deno.cache.v2.6",
    ],
);
pub const LIB_DENO_WORKER_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.worker.v2.5",
    &[LIB_DENO_WORKER_V2_5_INDEX_D_DS],
    &[
        "deno.ns.v2.5",
        "deno.shared_globals.v2.5",
        "esnext",
        "deno.cache.v2.5",
    ],
);

// deno.broadcast_channel
pub const LIB_DENO_BROADCAST_CHANNEL: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.broadcast_channel",
    &[LIB_DENO_BROADCAST_CHANNEL_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_BROADCAST_CHANNEL_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.broadcast_channel.v2.6",
    &[LIB_DENO_BROADCAST_CHANNEL_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_BROADCAST_CHANNEL_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.broadcast_channel.v2.5",
    &[LIB_DENO_BROADCAST_CHANNEL_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.cache
pub const LIB_DENO_CACHE: BuiltinLib =
    BuiltinLib::ambient_lib("deno.cache", &[LIB_DENO_CACHE_V2_6_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_CACHE_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.cache.v2.6",
    &[LIB_DENO_CACHE_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_CACHE_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.cache.v2.5",
    &[LIB_DENO_CACHE_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.canvas
pub const LIB_DENO_CANVAS: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.canvas",
    &[LIB_DENO_CANVAS_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_CANVAS_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.canvas.v2.6",
    &[LIB_DENO_CANVAS_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_CANVAS_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.canvas.v2.5",
    &[LIB_DENO_CANVAS_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.console
pub const LIB_DENO_CONSOLE: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.console",
    &[LIB_DENO_CONSOLE_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_CONSOLE_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.console.v2.6",
    &[LIB_DENO_CONSOLE_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_CONSOLE_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.console.v2.5",
    &[LIB_DENO_CONSOLE_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.crypto
pub const LIB_DENO_CRYPTO: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.crypto",
    &[LIB_DENO_CRYPTO_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_CRYPTO_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.crypto.v2.6",
    &[LIB_DENO_CRYPTO_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_CRYPTO_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.crypto.v2.5",
    &[LIB_DENO_CRYPTO_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.fetch
pub const LIB_DENO_FETCH: BuiltinLib =
    BuiltinLib::ambient_lib("deno.fetch", &[LIB_DENO_FETCH_V2_6_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_FETCH_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.fetch.v2.6",
    &[LIB_DENO_FETCH_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_FETCH_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.fetch.v2.5",
    &[LIB_DENO_FETCH_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.url
pub const LIB_DENO_URL: BuiltinLib =
    BuiltinLib::ambient_lib("deno.url", &[LIB_DENO_URL_V2_6_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_URL_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.url.v2.6",
    &[LIB_DENO_URL_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_URL_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.url.v2.5",
    &[LIB_DENO_URL_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.web
pub const LIB_DENO_WEB: BuiltinLib =
    BuiltinLib::ambient_lib("deno.web", &[LIB_DENO_WEB_V2_6_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_WEB_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.web.v2.6",
    &[LIB_DENO_WEB_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_WEB_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.web.v2.5",
    &[LIB_DENO_WEB_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.webgpu
pub const LIB_DENO_WEBGPU: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.webgpu",
    &[LIB_DENO_WEBGPU_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_WEBGPU_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.webgpu.v2.6",
    &[LIB_DENO_WEBGPU_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_WEBGPU_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.webgpu.v2.5",
    &[LIB_DENO_WEBGPU_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.websocket
pub const LIB_DENO_WEBSOCKET: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.websocket",
    &[LIB_DENO_WEBSOCKET_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_WEBSOCKET_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.websocket.v2.6",
    &[LIB_DENO_WEBSOCKET_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_WEBSOCKET_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.websocket.v2.5",
    &[LIB_DENO_WEBSOCKET_V2_5_INDEX_D_DS],
    &["esnext"],
);

// deno.webstorage
pub const LIB_DENO_WEBSTORAGE: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.webstorage",
    &[LIB_DENO_WEBSTORAGE_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_WEBSTORAGE_V2_6: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.webstorage.v2.6",
    &[LIB_DENO_WEBSTORAGE_V2_6_INDEX_D_DS],
    &["esnext"],
);
pub const LIB_DENO_WEBSTORAGE_V2_5: BuiltinLib = BuiltinLib::ambient_lib(
    "deno.webstorage.v2.5",
    &[LIB_DENO_WEBSTORAGE_V2_5_INDEX_D_DS],
    &["esnext"],
);
