import { defineService } from "@destack/service";
import { setting } from "../object/index.ts";

/** Setting values served as objects. */
export const settingService = defineService("setting", { objects: { setting } });
