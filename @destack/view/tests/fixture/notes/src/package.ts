import { t } from "@destack/locale";
import { definePackage } from "@destack/object";

/** The handle stacks import to install the notes app. */
export default definePackage({
    title: t`Notes`,
    description: t`Keep notebooks.`,
    icon: "notebook",
    color: "orange",
});
