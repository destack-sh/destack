import { expect, onTestFinished, test } from "@destack/test";
import { asc } from "../sql/index.ts";
import { TEST_DIALECTS, TestDatabase } from "./database.ts";
import { tally, tallyTemplate } from "./templates.ts";

test.for(TEST_DIALECTS)(
    "start each copy of a template's image from its rows and value, keeping every copy's writes its own, on %s",
    async (dialect) => {
        // copy the template twice and write into the first copy only
        const copy = async () => {
            const { storage, value } = await TestDatabase.copy(dialect, tallyTemplate);
            onTestFinished(() => storage.close());

            return { database: storage.database, value };
        };
        const first = await copy();
        await first.database.insert(tally).values({ name: "first", value: 2 });
        const second = await copy();

        // read the template's row in both copies, and only the first copy's own row in it
        expect({
            values: [first.value, second.value],
            first: await first.database.select().from(tally).orderBy(asc(tally.name)),
            second: await second.database.select().from(tally).orderBy(asc(tally.name)),
        }).toEqual({
            values: ["started", "started"],
            first: [
                { name: "first", value: 2 },
                { name: "started", value: 1 },
            ],
            second: [{ name: "started", value: 1 }],
        });
    },
);
