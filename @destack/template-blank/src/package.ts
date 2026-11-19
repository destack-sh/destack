import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { home } from "./view/index.ts";

/** The handle stacks import to install this package. */
export default definePackage({
    title: t`Blank`,
    description: t`Start a new Destack package from an empty package with one setting and one page.`,
    icon: "file-dashed",
    color: "sky",
    views: [home],
});
