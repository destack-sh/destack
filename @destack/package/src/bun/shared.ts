import type { ModuleLoad } from "./program.ts";

/** The modules a compiled program shares, as its build generates them, none from sources. */
export const SHARED_MODULES: Readonly<Record<string, ModuleLoad>> | undefined = undefined;
