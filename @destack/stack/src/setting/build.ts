import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";

/** The builds of each package source a space retains by default: ten. */
const RETAINED_BUILDS = 10;

/** How many latest succeeded builds of each package source a space retains. */
export const buildRetention = defineSetting({
    name: "build.retention",
    title: "Build retention",
    description:
        "How many of the latest succeeded builds of each package source a space keeps, beside every build a revision applies.",
    schema: schema.number().int().min(1).max(1000),
    default: RETAINED_BUILDS,
    scope: "space",
    overrides: [],
    apply: "immediate",
});
