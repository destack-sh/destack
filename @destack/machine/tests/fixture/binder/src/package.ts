import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { binder, sheet } from "./binder.ts";

/** The binders: sheets read through binders that keep no history. */
export default definePackage({
    title: t`Binders`,
    description: t`Keep sheets in binders.`,
    icon: "folder",
    color: "iris",
    objects: [binder, sheet],
});
