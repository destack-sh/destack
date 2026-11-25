import { t } from "@destack/locale";
import { definePackage } from "@destack/object";

/** The handle stacks import to install this package. */
export default definePackage({
    title: t`App`,
    description: t`Show an empty page with Destack's mark: the app a space and a new package start from.`,
    icon: "file-dashed",
    color: "orange",
});
