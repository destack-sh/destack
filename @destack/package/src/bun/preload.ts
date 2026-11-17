import { join } from "node:path";
import { BunEnvironment } from "@destack/environment/bun";
import { TransformCache } from "../build/build.ts";
import { ModuleTransformer } from "../transform/transform.ts";
import { modulePlugin } from "./plugin.ts";
import { Version } from "@destack/schema";

/** The process's environment, whose files the stamps read and the cache stores. */
const environment = BunEnvironment.system;

/** The directory in the person's cache that stores stamped modules across processes. */
const cache = join(await environment.files.cacheDirectory(), "destack", "modules");

/** The version a release gives the source this process runs, absent outside a release. */
const release = environment.process.variables["DESTACK_RELEASE_VERSION"];

/** The transformer stamping every package source this process loads, at the release's version. */
const transformer = new ModuleTransformer(environment, {
    cache: TransformCache.directory(cache, environment),
    ...(release === undefined ? {} : { source: Version.parse(release) }),
});

// inject module metadata into every package source loaded by this process
await environment.process.runtime.plugin(modulePlugin(transformer));
