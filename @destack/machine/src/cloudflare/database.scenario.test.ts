import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { Miniflare } from "miniflare";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { bundleWorker } from "./test/bundle.ts";

/** The counts the scenario answers. */
const Measured = schema.object({
    space: schema.object({ measured: schema.number(), own: schema.number() }),
    installation: schema.object({ measured: schema.number(), own: schema.number() }),
});

/** The space whose object copies its installations' databases, as the scenario declares it. */
const SPACE = "space-01996ab0-0000-7000-8000-00000000e001";

/** What the copy scenario answers. */
const Copied = schema.object({
    pages: schema.number(),
    entries: schema.array(schema.object({ id: schema.string(), title: schema.string() })),
    stray: schema.number(),
});

/** Run the scenario in the Workers runtime with a space's object and installations' facets. */
async function start(): Promise<Miniflare> {
    // bundle the scenario with its declarations' module metadata
    const script = await bundleWorker(new URL("./test/scenario.ts", import.meta.url));

    // run it with the partitions' one class
    const worker = new Miniflare({
        modules: true,
        script,
        compatibilityDate: WORKERD_COMPATIBILITY_DATE,
        compatibilityFlags: ["nodejs_compat"],
        bindings: { DESTACK_JURISDICTION: "none" },
        durableObjects: {
            PARTITION: { className: "Partition", useSQLite: true },
        },
    });
    onTestFinished(() => worker.dispose());

    return worker;
}

test("measure a space's own object as the space's databases and an installation's facet as the installation's", async () => {
    const worker = await start();

    // address the space's object
    const namespace = await worker.getDurableObjectNamespace("PARTITION");
    const space = namespace.get(namespace.idFromName(SPACE));
    const response = await space.fetch("https://space/");
    const { space: own, installation } = Measured.parse(await response.json());

    // count each object's whole storage
    expect({
        space: own.measured,
        installation: installation.measured,
        isApart: installation.own > own.own,
    }).toEqual({ space: own.own, installation: installation.own, isApart: true });
});

test("snapshot an installation's database in its facet through the space's store, and restore it into another installation's facet, refusing a page outside any copy", async () => {
    const worker = await start();

    // copy the database through the space's object
    const namespace = await worker.getDurableObjectNamespace("PARTITION");
    const space = namespace.get(namespace.idFromName(SPACE));
    const response = await space.fetch("https://space/copy");
    const copied = Copied.parse(await response.json());

    // read the entries back from the second object, the store with the manifest and a page
    expect(copied).toEqual({
        pages: 2,
        entries: [
            { id: "a", title: "Kept" },
            { id: "b", title: "Moved" },
        ],
        stray: 401,
    });
});
