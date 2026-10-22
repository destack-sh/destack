import { expect, onTestFinished, test } from "@destack/test";
import { asc } from "../sql/index.ts";
import { TestDatabase } from "./database.ts";
import { tally, tallyTemplate } from "./templates.ts";

/** Copy the tally template for one test, closed when it finishes. */
async function copy() {
    const { storage, value } = await TestDatabase.copy(tallyTemplate);
    onTestFinished(() => storage.close());

    return { database: storage.database, value };
}

test("start each copy of a template's image from its rows and value, every copy's writes separate", async () => {
    // copy the template twice and write into the first copy only
    const first = await copy();
    await first.database.insert(tally).values({ name: "first", value: 2 });
    const second = await copy();

    // read the template's row in both copies, and the first copy's row only in it
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
});
