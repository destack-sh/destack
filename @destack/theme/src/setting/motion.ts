import { defineSetting } from "@destack/setting/declare";
import { DEFAULT_PREFERENCES, Motion } from "../theme/index.ts";

/** The motion a person sees, overridden per package, space, installation or client. */
export const motion = defineSetting({
    name: "motion",
    title: "Motion",
    description: "Use the system motion preference, or show or remove transitions.",
    schema: Motion,
    default: DEFAULT_PREFERENCES.motion,
    scope: "user",
    overrides: ["package", "space", "installation", "client"],
    apply: "immediate",
});
