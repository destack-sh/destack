import { expect, onTestFinished, test } from "@destack/test";
import { asc } from "../sql/index.ts";
import { TestDatabase } from "./database.ts";
import { pairTemplate, tally, tallyTemplate } from "./templates.ts";

/** Copy the tally template for one test, closed when it finishes. */
async function copy() {
    const { simulation, databases, value } = await TestDatabase.copy(tallyTemplate, {
        label: "copy",
    });
    onTestFinished(async () => {
        await databases.tally.close();
        await simulation[Symbol.asyncDispose]();
    });

    return { database: databases.tally.database, value };
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

test("copy a template's databases as one set, each with the rows its write left in it", async () => {
    // copy the pair template, whose write counts one database's rows into the other
    const { simulation, databases, value } = await TestDatabase.copy(pairTemplate, {
        label: "copy",
    });
    onTestFinished(async () => {
        await databases.counted.close();
        await databases.counting.close();
        await simulation[Symbol.asyncDispose]();
    });

    // read each database's rows and the shared value
    expect({
        value,
        counted: await databases.counted.database.select().from(tally).orderBy(asc(tally.name)),
        counting: await databases.counting.database.select().from(tally),
    }).toEqual({
        value: 2,
        counted: [
            { name: "first", value: 1 },
            { name: "second", value: 2 },
        ],
        counting: [{ name: "counted", value: 2 }],
    });
});
