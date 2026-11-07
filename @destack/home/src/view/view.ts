import { space } from "@destack/account/object";
import { setting } from "@destack/setting/object";
import { defineView } from "@destack/view/declare";

/** The start view: the account's spaces with the views their apps mount. */
export const home = defineView({
    name: "home",
    categories: ["home"],
    objects: [space],
    component: () => import("./app.tsx"),
});

/** The person's settings, stored in their own scope and copied into their home. */
export const settings = defineView({
    name: "settings",
    objects: [setting],
    component: () => import("./settings.tsx"),
});
