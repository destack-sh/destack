import { schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { defineSetting } from "@destack/setting/declare";
import { shortcut } from "@destack/view/declare";

/** A link under a release channel's scheme, such as `destack://notes--personal--florian/home`. */
const LinkText = schema.string().regex(/^destack(?:-[a-z]+)?:\/\/\S+$/u);

/** The link a new window opens, Home's start view by default. */
export const start = defineSetting({
    name: "start",
    title: "Start link",
    description: "The view or object a new window opens.",
    schema: LinkText.nullable(),
    default: null,
    scope: "user",
    overrides: ["device"],
    apply: "immediate",
});

/** Whether the desktop opens at login on this computer. */
export const autostart = defineSetting({
    name: "autostart",
    title: "Open at login",
    description: "Open the desktop when you log in to this computer.",
    schema: schema.boolean(),
    default: true,
    scope: "device",
    overrides: [],
    apply: "immediate",
});

/** The settings of the desktop, beside the native device's settings. */
export const DESKTOP_SETTINGS: readonly Setting[] = [start, shortcut, autostart];
