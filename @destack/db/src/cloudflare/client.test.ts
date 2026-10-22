import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/build";
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
        compatibilityDate: WORKERD_COMPATIBILITY_DATE,
        compatibilityFlags: ["nodejs_compat"],
        durableObjects: { NOTES: { className: "Notes", useSQLite: true } },
    });
    onTestFinished(() => worker.dispose());

    return worker;
}

test("migrate, write and read a Durable Object's SQLite storage, rolling back only a failed nested transaction", async () => {
    const worker = await start();

    // commit the written and the outer rows, without the nested one
    const response = await worker.dispatchFetch("https://notes.test/");
    expect([response.status, await response.json()]).toEqual([
        200,
        [
            { id: "a", title: "Changed" },
            { id: "b", title: "Outer" },
        ],
    ]);
});

test("store two databases of one declaration in a Durable Object's storage, each namespace with separate rows, log and plan", async () => {
    const worker = await start();

    // read each namespace's row and logged insert, and a plan dropping only its table
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

    // set up the database in its object, then write, measure and drop its relations
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

test("write and read rows in statements binding more values than a Durable Object's storage takes at once, or integers it cannot bind", async () => {
    const worker = await start();

    // write 80 rows in one statement of 160 values and read them back intact in one
    const response = await worker.dispatchFetch("https://notes.test/parameters");
    expect([response.status, await response.json()]).toEqual([
        200,
        { isEqual: true, stored: { size: "4611686018427387904" } },
    ]);
});

test("refuse a transaction's client used past it, even while a later transaction is open", async () => {
    const worker = await start();

    // commit only the later transaction's write
    const response = await worker.dispatchFetch("https://notes.test/leaked");
    expect([response.status, await response.json()]).toEqual([
        200,
        {
            refused: "the transaction has finished",
            rows: [
                { id: "a", title: "Kept" },
                { id: "b", title: "Later" },
            ],
        },
    ]);
});

test("count the rows a Durable Object's statements read and write, each in the one measure following them", async () => {
    const worker = await start();

    // count the insert's rows with key index entries, then the three notes read back
    const response = await worker.dispatchFetch("https://notes.test/rows");
    expect([response.status, await response.json()]).toEqual([
        200,
        {
            counts: [
                [0, 6],
                [3, 0],
                [0, 0],
            ],
            notes: 3,
        },
    ]);
});

test("read another object's database through its reader, refusing other operations and undoing a statement that writes", async () => {
    const worker = await start();

    // read a note, a file and a namespace's note, and refuse a measure and a delete
    const response = await worker.dispatchFetch("https://notes.test/reader");
    expect([response.status, await response.json()]).toEqual([
        200,
        {
            found: { id: "b", title: "Note b" },
            named: { id: "n", title: "Spaced" },
            measured: "this object answers reads alone, not measure",
            files: [["f", "9007199254740991", "0001feff"]],
            deleted: "a read of another object wrote rows",
            kept: ["a", "b"],
        },
    ]);
});
