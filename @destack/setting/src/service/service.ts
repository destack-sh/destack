import { defineService } from "@destack/service";
import { setting } from "./setting.ts";
import { assignment } from "./assignment.ts";
import { policy } from "./policy.ts";

/** Setting discovery, effective values and revisioned administration. */
export const settingService = defineService("setting", { setting, assignment, policy });
