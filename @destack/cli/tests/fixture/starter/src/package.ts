import { t } from "@destack/locale";
import { definePackage } from "@destack/object";

/** The package Example starts new packages from. */
export default definePackage({
    title: t`Starter`,
    description: t`Start a package of Example's.`,
    icon: "file-dashed",
    color: "cyan",
});
