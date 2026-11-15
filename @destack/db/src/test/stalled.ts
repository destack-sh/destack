import { schema } from "@destack/schema";
import { TestTemplate } from "./template.ts";

/** A template whose build never finishes, as a broken definition's. */
export const stalledTemplate = new TestTemplate({
    name: "db.stalled",
    databases: {},
    value: schema.null(),
    world: { seed: 1, start: Date.UTC(2026, 9, 1) },
    build: () => new Promise<null>(() => {}),
});
