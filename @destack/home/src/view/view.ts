import { space } from "@destack/account/object";
import { setting } from "@destack/setting/object";
import { defineView } from "@destack/view/declare";

/** The start view: the account's spaces with the views their apps mount. */
export const home = defineView({
    name: "home",
    categories: ["main", "home"],
    objects: [space],
    component: () => import("./home.tsx"),
});

/** The person's settings, stored in their own scope and copied into their home. */
export const settings = defineView({
    name: "settings",
    objects: [setting],
    component: () => import("./settings.tsx"),
});

/** The command palette over the commands and views of the person's installations. */
export const command = defineView({
    name: "command",
    categories: ["command"],
    component: () => import("../palette/palette.tsx"),
});

/** The universe's start: the signed-in person's Home, else the sign-in. */
export const start = defineView({
    name: "start",
    audience: "anyone",
    component: () => import("../sign-in/start.tsx"),
});

/** The universe's sign-in: a provider or a mailed code, then the second factor. */
export const signIn = defineView({
    name: "sign-in",
    audience: "anyone",
    component: () => import("../sign-in/sign-in.tsx"),
});

/** The universe's sign-up: name, handle and residency, then the mailed code. */
export const signUp = defineView({
    name: "sign-up",
    audience: "anyone",
    component: () => import("../sign-in/sign-up.tsx"),
});

/** An application's request for access, allowed or denied by the person. */
export const consent = defineView({
    name: "consent",
    audience: "anyone",
    component: () => import("../sign-in/consent.tsx"),
});

/** The apps the person lets act for them, each removable. */
export const apps = defineView({
    name: "apps",
    audience: "anyone",
    component: () => import("../sign-in/apps.tsx"),
});

/** A device's sign-in by the code it shows. */
export const verification = defineView({
    name: "verification",
    audience: "anyone",
    component: () => import("../sign-in/verification.tsx"),
});
