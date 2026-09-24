/// <reference types="bun" />
import { modulePlugin } from "./bun.ts";

// inject module metadata into every package source loaded by this process
await Bun.plugin(modulePlugin);
