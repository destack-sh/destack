import { definePackage } from "@destack/object";
import { setting } from "./object/index.ts";
import { SettingError } from "./error/error.ts";

/** The handle stores serve setting values through, and clients call them by. */
export default definePackage({
    objects: { setting },
    errors: SettingError,
});
