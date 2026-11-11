import type { Setting } from "@destack/setting";
import { defineSetting } from "@destack/setting/declare";
import { schema } from "@destack/schema";
import { t } from "@destack/locale";

/** The space a computer's terminal acts in without a --space, as `destack switch` sets it. */
export const currentSpace = defineSetting({
    name: "space",
    title: t`Space`,
    description: t`The space the terminal acts in when a command names none.`,
    schema: schema.identifier("space").nullable(),
    default: null,
    scope: "device",
});

/** The login a computer's terminal and desktop act as while several people are signed in. */
export const currentLogin = defineSetting({
    name: "login",
    title: t`Signed-in person`,
    description: t`The person the terminal and the desktop act as while several are signed in.`,
    schema: schema.identifier("login").nullable(),
    default: null,
    scope: "device",
});

/** The settings of a computer's native device, which it stores as local objects. */
export const NATIVE_SETTINGS: readonly Setting[] = [currentSpace, currentLogin];
