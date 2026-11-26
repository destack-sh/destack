import { t } from "@destack/locale";
import { definePackage } from "@destack/object";

/** The handle stacks import to install this package. */
export default definePackage({
    title: t.context("title")`Template`,
    description: t`Show an empty page with Destack's mark.`,
    icon: "file-dashed",
    color: "orange",
});
