import { schema } from "@destack/schema";
import { integer, text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { join } from "node:path";
import { testHost } from "@destack/test/host";
import { TestTemplate } from "./template.ts";

/** The moment the templates' builds run at: 2026-10-01T00:00:00Z. */
const WRITTEN_AT = Date.UTC(2026, 9, 1);

/** The seed of the templates' worlds. */
const SEED = 1;

/** Counters by name. */
export const tally = defineTable("template_tally", {
    /** The counter's name. */
    name: text("name").primaryKey(),
    /** The counted value. */
    value: integer("value").notNull(),
});

/** One counter, which the template's forks start with. */
export const tallyTemplate = new TestTemplate({
    name: "db.tally",
    databases: { tally: [tally] },
    value: schema.literal("started"),
    world: { seed: SEED, start: WRITTEN_AT },
    build: async ({ databases }) => {
        // write the one counter
        await databases.tally.database.insert(tally).values({ name: "started", value: 1 });

        return "started" as const;
    },
});

/** Two counters in two databases written together, the second counting the first's rows. */
export const pairTemplate = new TestTemplate({
    name: "db.pair",
    databases: { counted: [tally], counting: [tally] },
    value: schema.number(),
    world: { seed: SEED, start: WRITTEN_AT },
    build: async ({ databases: { counted, counting } }) => {
        // write two rows, and count them into the second database
        await counted.database.insert(tally).values([
            { name: "first", value: 1 },
            { name: "second", value: 2 },
        ]);
        const rows = await counted.database.select().from(tally);
        await counting.database.insert(tally).values({ name: "counted", value: rows.length });

        return rows.length;
    },
});

/** A second counter on the tally template's image, named by what the tally's build returned. */
export const grownTemplate = new TestTemplate({
    name: "db.grown",
    from: tallyTemplate,
    value: schema.string(),
    build: async ({ databases, parent }) => {
        // add a counter beside the parent's
        const from = schema.string().parse(parent);
        await databases.tally.database.insert(tally).values({ name: "grown", value: 2 });

        return `grown from ${from}`;
    },
});

/** The name of the file the shelf's build writes into its directory. */
export const SHELF_FILE = "shelf.txt";

/** A counter in a database the build opens by a key, and a file in its directory. */
export const shelfTemplate = new TestTemplate({
    name: "db.shelf",
    databases: {},
    volume: [tally],
    value: schema.null(),
    world: { seed: SEED, start: WRITTEN_AT },
    build: async ({ open, directory, simulation }) => {
        // write a counter into a volume and a line into a file
        const volume = await open("shelf");
        await volume.database.insert(tally).values({ name: "shelved", value: 1 });
        await simulation
            .environment("shelf")
            .scheduler.run(() =>
                testHost().files.writeTextFile(join(directory, SHELF_FILE), "first\n"),
            );

        return null;
    },
});

/** A second counter in the shelf's volume, a second volume, and a second line in its file. */
export const stackedTemplate = new TestTemplate({
    name: "db.stacked",
    from: shelfTemplate,
    value: schema.null(),
    build: async ({ open, directory, simulation }) => {
        // add to the parent's volume and file, and open a volume of its own
        const shelf = await open("shelf");
        await shelf.database.insert(tally).values({ name: "stacked", value: 2 });
        const own = await open("own");
        await own.database.insert(tally).values({ name: "own", value: 3 });
        const file = join(directory, SHELF_FILE);
        const { files } = testHost();
        await simulation
            .environment("shelf")
            .scheduler.run(async () =>
                files.writeTextFile(file, `${await files.readTextFile(file)}second\n`),
            );

        return null;
    },
});

/** A template whose build fails, and whose database then fails to close. */
export const failingTemplate = new TestTemplate({
    name: "db.failing",
    databases: { tally: [tally] },
    value: schema.null(),
    world: { seed: SEED, start: WRITTEN_AT },
    build: async ({ databases }) => {
        // fail the database's close, then the build
        databases.tally.close = async () => {
            throw new Error("connection is closing");
        };
        throw new Error("the build failed");
    },
});
