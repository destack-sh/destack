import { schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { defineSetting } from "@destack/setting/declare";
import { shortcut, start } from "@destack/view/setting";
import { t } from "@destack/locale";

/** Whether the desktop opens at login on this computer. */
export const autostart = defineSetting({
    name: "autostart",
    title: t`Open at login`,
    description: t`Open the desktop when you log in to this computer.`,
    schema: schema.boolean(),
    default: true,
    scope: "device",
});

/** The settings of the desktop, beside the native device's settings. */
export const DESKTOP_SETTINGS: readonly Setting[] = [start, shortcut, autostart];
