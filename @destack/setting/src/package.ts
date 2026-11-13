import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { setting } from "./object/index.ts";
import { SettingError } from "./error/error.ts";

/** The handle stores serve setting values through, and clients call them by. */
export default definePackage({
    title: t`Settings`,
    description: t`Declare, place and resolve scoped settings.`,
    icon: "gear-six",
    color: "violet",
    objects: [setting],
    errors: SettingError,
});
