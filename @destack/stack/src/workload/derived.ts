import { VIEW_WORKLOAD } from "@destack/view/declare";
import { HOOK_WORKLOAD } from "./hook.ts";

/** The workloads a build derives from a package's declarations, whose names no package declares. */
export const DERIVED_WORKLOADS: ReadonlySet<string> = new Set([HOOK_WORKLOAD, VIEW_WORKLOAD]);
