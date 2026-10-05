/// <reference types="bun" />
import { registerModulePlugin } from "./plugin.ts";

// inject module metadata into every package source loaded by this process
registerModulePlugin();
