import { Miniflare } from "miniflare";
import { fileURLToPath } from "node:url";
import { modulePlugin } from "@destack/package/bun";
import { expect, onTestFinished, test } from "@destack/test";

/** Run the scenario in the Workers runtime with one Durable Object, disposed after the test. */
async function start(): Promise<Miniflare> {
    // bundle the scenario with its declarations' module metadata
    const compiled = await Bun.build({
        entrypoints: [fileURLToPath(new URL("./test/scenario.ts", import.meta.url))],
        format: "esm",
        target: "browser",
        conditions: ["workerd", "worker", "browser"],
        external: ["node:*", "cloudflare:*"],
        plugins: [modulePlugin],
    });
    const [output] = compiled.outputs;
    if (output === undefined) {
        throw new Error("the scenario bundled into no file");
    }

    // run it with one Durable Object
    const worker = new Miniflare({
        modules: true,
        script: await output.text(),
        compatibilityDate: "2026-07-30",
        compatibilityFlags: ["nodejs_compat"],
        durableObjects: { NOTES: { className: "Notes", useSQLite: true } },
    });
    onTestFinished(() => worker.dispose());

    return worker;
}

test("migrate, write and read a Durable Object's SQLite storage, rolling back only a failed nested transaction", async () => {
    const worker = await start();

    // keep the written and the outer rows, without the inner one
    const response = await worker.dispatchFetch("https://notes.test/");
    expect([response.status, await response.json()]).toEqual([
        200,
        [
            { id: "a", title: "Changed" },
            { id: "b", title: "Outer" },
        ],
    ]);
});

test("keep two databases of one declaration in a Durable Object's storage, each namespace with its own rows, log and plan", async () => {
    const worker = await start();

    // read each namespace's own row, its own logged insert in its own scope, and a plan dropping only its own table
    const response = await worker.dispatchFetch("https://notes.test/namespaces");
    expect([response.status, await response.json()]).toEqual([
        200,
        [1, 2].map((index) => ({
            rows: [{ id: "a", title: `Entry ${index}` }],
            changes: [["insert", `space-${index}`, { id: "a", title: `Entry ${index}` }]],
            steps: [`table/${index === 1 ? "first" : "second"}.destack__durable_scenario__entry`],
        })),
    ]);
});

test("provision, plan, apply, measure and destroy a workload's database in a Durable Object, opening it through the workload's connector", async () => {
    const worker = await start();

    // name the database in its object, create its table, keep the written row, count the object's bytes and drop every relation
    const response = await worker.dispatchFetch("https://notes.test/host");
    expect([response.status, await response.json()]).toEqual([
        200,
        {
            reference: "durable-object:database-01996ab0-0000-7000-8000-00000000d002",
            steps: [
                "table/database-01996ab0-0000-7000-8000-00000000d002.destack__durable_scenario__entry",
            ],
            rows: [{ id: "a", title: "Kept" }],
            isMeasured: true,
            remaining: [{ count: 0 }],
        },
    ]);
});

test("write and read rows in statements binding more values than a Durable Object's storage takes at once", async () => {
    const worker = await start();

    // keep 80 rows written in one statement of 160 values and read in one of 80, quotes and markers in their text intact
    const response = await worker.dispatchFetch("https://notes.test/parameters");
    expect([response.status, await response.json()]).toEqual([200, { isEqual: true }]);
});

test("read and write another database of a Durable Object's storage from within a transaction of one, joining it", async () => {
    const worker = await start();

    // read the first database's note and both of the second's, the joined write included
    const response = await worker.dispatchFetch("https://notes.test/joined");
    expect([response.status, await response.json()]).toEqual([
        200,
        [
            [{ id: "a", title: "Kept" }],
            [
                { id: "a", title: "Kept" },
                { id: "b", title: "Joined" },
            ],
        ],
    ]);
});

test("count the rows a Durable Object's statements write and read, each in the one measure following them", async () => {
    const worker = await start();

    // count the insert's rows as workerd counts them, its key index entries included, the three notes read back, and nothing after
    const response = await worker.dispatchFetch("https://notes.test/rows");
    expect([response.status, await response.json()]).toEqual([
        200,
        {
            counts: [
                [17, 8],
                [3, 0],
                [0, 0],
            ],
            notes: 3,
        },
    ]);
});
