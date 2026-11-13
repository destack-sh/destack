import { BORN_SPACE, ids, TestSpace } from "@destack/machine/test";
import { Forge } from "@destack/registry/client";
import { FOLLOW_TEMPLATE, FORGE_TEMPLATE } from "../src/test/index.ts";

/** The forge a template's space follows, claiming no names: it selects no repository or tag. */
const TEMPLATE_FORGE = new Forge(
    { resolve: async () => new Map() },
    async () => ids.space,
    ids.machine,
);

/** The templates forge's tests copy, which the run writes once. */
export const templates = [
    FORGE_TEMPLATE,
    FOLLOW_TEMPLATE,
    // follow the forge's references and tags, as a building machine's space does
    TestSpace.template({ follows: TEMPLATE_FORGE.follows() }, BORN_SPACE),
];
