import { join } from "node:path";
import { BunEnvironment } from "@destack/environment/bun";
import { TransformCache } from "../build/build.ts";
import { ModuleTransformer } from "../transform/transform.ts";
import { modulePlugin } from "./plugin.ts";

/** The process's environment, whose files the stamps read and the cache stores. */
const environment = BunEnvironment.system;

/** The directory in the person's cache that stores stamped modules across processes. */
const cache = join(await environment.files.cacheDirectory(), "destack", "modules");

/** The transformer stamping every package source this process loads. */
const transformer = new ModuleTransformer(environment, {
    cache: TransformCache.directory(cache, environment),
});

// inject module metadata into every package source loaded by this process
await environment.process.runtime.plugin(modulePlugin(transformer));
