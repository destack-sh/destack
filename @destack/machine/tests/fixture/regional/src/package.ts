import { t } from "@destack/locale";
import { definePackage } from "@destack/object";

/** A package with one required setting. */
export default definePackage({
    title: t`Regional`,
    description: t`Serve a region.`,
    icon: "globe",
    color: "teal",
});
