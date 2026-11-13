import { t } from "@destack/locale";
import { definePackage } from "@destack/object";

/** The handle stacks import to install this package. */
export default definePackage({
    title: t`Blank`,
    description: t`Start a new Destack package from an empty package with one setting.`,
    icon: "file-dashed",
    color: "sky",
});
