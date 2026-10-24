import { BunEnvironment } from "@destack/environment/bun";
import { modulePlugin } from "./plugin.ts";

// inject module metadata into every package source loaded by this process
await BunEnvironment.system.process.runtime.plugin(modulePlugin(BunEnvironment.system.files));
