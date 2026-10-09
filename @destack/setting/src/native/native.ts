import { SpaceAddress } from "@destack/account/address";
import { schema } from "@destack/schema";
import { defineSetting } from "../declare/index.ts";
import { SettingCatalog } from "../setting/index.ts";

/** The space a computer's terminal acts in without a --space, as `destack switch` sets it. */
export const space = defineSetting({
    name: "space",
    title: "Space",
    description: "The space the terminal acts in when a command names none.",
    schema: SpaceAddress.nullable(),
    default: null,
    scope: "client",
    overrides: [],
    apply: "immediate",
});

/** The login a computer's terminal and desktop act as while several people are signed in. */
export const login = defineSetting({
    name: "login",
    title: "Signed-in person",
    description: "The person the terminal and the desktop act as while several are signed in.",
    schema: schema.identifier("login").nullable(),
    default: null,
    scope: "client",
    overrides: [],
    apply: "immediate",
});

/** The settings of a computer's native client, which it keeps as local objects. */
export const NATIVE_SETTINGS = new SettingCatalog([space, login]);
