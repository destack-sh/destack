import { expect, onTestFinished, test } from "@destack/test";
import { asc } from "../sql/index.ts";
import { TestRun } from "./run.ts";
import { join } from "node:path";
import { present } from "@destack/schema";
import { testHost } from "@destack/test/host";
import {
    grownTemplate,
    pairTemplate,
    SHELF_FILE,
    shelfTemplate,
    stackedTemplate,
    tally,
    tallyTemplate,
} from "./templates.ts";

/** Fork the tally template's image for one test, closed when it finishes. */
async function copy() {
    const image = await TestRun.image(tallyTemplate);
    const { simulation, databases, value } = await image.fork("copy");
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

test("keep the log position the image left in every copy, so followers recorded in it resume", async () => {
    // copy the template twice, writing nothing
    const first = await copy();
    const second = await copy();

    // read the same log, epoch and sequence in both
    expect(await first.database.log.position()).toEqual(await second.database.log.position());
});

test("copy a template's databases as one set, each with the rows its write left in it", async () => {
    // copy the pair template, whose write counts one database's rows into the other
    const image = await TestRun.image(pairTemplate);
    const { simulation, databases, value } = await image.fork("copy");
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

test("build a template on its parent's image, with the parent's rows and value and its own beside them", async () => {
    // fork the template built on the tally's image, and the tally's own image
    const grown = await (await TestRun.image(grownTemplate)).fork("grown");
    const parent = await (await TestRun.image(tallyTemplate)).fork("parent");
    onTestFinished(async () => {
        await grown.databases.tally.close();
        await parent.databases.tally.close();
        await grown.simulation[Symbol.asyncDispose]();
        await parent.simulation[Symbol.asyncDispose]();
    });

    // read the parent's counter and the added one in the child, the parent's image left as it was
    expect({
        value: grown.value,
        grown: await grown.databases.tally.database.select().from(tally).orderBy(asc(tally.name)),
        parent: await parent.databases.tally.database.select().from(tally),
    }).toEqual({
        value: "grown from started",
        grown: [
            { name: "grown", value: 2 },
            { name: "started", value: 1 },
        ],
        parent: [{ name: "started", value: 1 }],
    });
});

test("carry a parent's volumes and files into the image built on it, leaving the parent's as they were", async () => {
    // build the image stacked on the shelf's, and fork both
    const stacked = await TestRun.image(stackedTemplate);
    const shelf = await TestRun.image(shelfTemplate);
    const forks = [await stacked.fork("stacked"), await shelf.fork("shelf")];
    onTestFinished(async () => {
        for (const fork of forks) {
            await Promise.all([...fork.volumes.values()].map((volume) => volume.close()));
            await fork.simulation[Symbol.asyncDispose]();
        }
    });

    // read each fork's volumes by key and each image's file
    const { files } = testHost();
    const read = await Promise.all(
        forks.map(async (fork) =>
            Object.fromEntries(
                await Promise.all(
                    [...fork.volumes].map(async ([key, volume]) => {
                        const rows = await volume.database.select().from(tally);

                        const names = rows.map((row) => row.name);

                        return [
                            key,
                            names.toSorted((left, right) => left.localeCompare(right)),
                        ] as const;
                    }),
                ),
            ),
        ),
    );
    expect({
        volumes: read,
        files: [
            await files.readTextFile(join(stacked.record.files, SHELF_FILE)),
            await files.readTextFile(join(shelf.record.files, SHELF_FILE)),
        ],
        hasForked: present(forks[0], "the stacked fork").volumes.size,
    }).toEqual({
        volumes: [{ own: ["own"], shelf: ["shelved", "stacked"] }, { shelf: ["shelved"] }],
        files: ["first\nsecond\n", "first\n"],
        hasForked: 2,
    });
});
