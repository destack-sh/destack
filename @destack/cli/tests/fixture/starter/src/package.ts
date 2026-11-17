import { t } from "@destack/locale";
import { definePackage } from "@destack/object";

/** The package Acme starts new packages from. */
export default definePackage({
    title: t`Starter`,
    description: t`Start a package of Acme's.`,
    icon: "file-dashed",
    color: "sky",
});
