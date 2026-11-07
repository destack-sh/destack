import { schema } from "@destack/schema";
import { TestTemplate } from "./template.ts";

/** The templates of a module whose one write never finishes, as a broken definition's writer. */
export const templates = [
    new TestTemplate({
        name: "db.stalled",
        databases: {},
        value: schema.null(),
        world: { seed: 1, start: Date.UTC(2026, 9, 1) },
        write: () => new Promise<null>(() => {}),
    }),
];
