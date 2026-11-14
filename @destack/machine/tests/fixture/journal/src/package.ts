import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { entry } from "./journal.ts";

/** The journal: entries whose history a space keeps. */
export default definePackage({
    title: t`Journal`,
    description: t`Keep a journal with its history.`,
    icon: "notebook",
    color: "iris",
    objects: [entry],
});
