import { BORN_SPACE, ids, TestSpace } from "@destack/stack/test";
import { Forge } from "../src/client/index.ts";
import { FORGE_TEMPLATE } from "../src/test/index.ts";

/** The forge a template's space follows, claiming no names: it selects no repository or tag. */
const TEMPLATE_FORGE = new Forge({ resolve: async () => new Map() }, async () => ids.space);

/** The templates forge's tests copy, which the run writes once. */
export const templates = [
    FORGE_TEMPLATE,
    // follow the forge's references and tags, as a building machine's space does
    TestSpace.template({ follows: TEMPLATE_FORGE.follows() }, BORN_SPACE),
];
