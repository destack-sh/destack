import { t } from "@destack/locale";
import { defineView } from "@destack/view/declare";

/** The package's start page, which its origin opens at its root. */
export const home = defineView({
    name: "home",
    title: t`Home`,
    categories: ["main"],
    component: () => import("./home.tsx"),
});
