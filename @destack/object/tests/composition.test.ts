import { principal } from "@destack/access";
import { defineDatabase, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { Environment } from "@destack/environment";
import { argument, plural, t } from "@destack/locale";
import { graph, PackageVersion } from "@destack/package";
import { MemoryBlobStorage } from "@destack/resource";
import { type Identifier, nth, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { describePackage } from "../src/inspect/index.ts";
import {
    defineObject,
    field,
    Isolate,
    OBJECT_KIND,
    ObjectDescription,
    ObjectType,
    ObjectTypePool,
    Package,
    PACKAGE_KIND,
    shareable,
    tablesOf,
} from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { ObjectStore } from "../src/server/index.ts";
import {
    testCallKey,
    testEvents,
    testIdentity,
    testServerContext,
    testSimulation,
} from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { spaceId } from "./fixture/device.ts";

/** A build declaration, as a isolate receives it. */
type Declaration = Pick<graph.Declaration, "symbol" | "kind" | "kindPackageId" | "description">;

/** The notebooks package. */
const notebooks = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000b1",
    name: "@example/notebooks",
    version: "2026.9.0",
});

/** The tasks package. */
const tasks = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000b2",
    name: "@example/tasks",
    version: "2026.9.0",
});

/** A named record of one package, as that package declares it. */
const named = (name: string, pluralName: string) =>
    defineObject(
        {
            name,
            plural: pluralName,
            title: t`${plural(argument("count"), { one: "Record", other: "Records" })}`,
            scope: space,
            fields: { title: field.string(schema.string().min(1)) },
            traits: [shareable({})],
        },
        (method) => ({
            create: method.create("manage"),
        }),
    );

/** A listing of titled objects. */
const Listed = schema.looseObject({
    /** The listed objects. */
    items: schema.array(schema.looseObject({ title: schema.string() })),
});

/** The space's main database resource, which applies every installation's tables. */
const MAIN_DATABASE = "database-01996ab0-0000-7000-8000-0000000000e1";

/** The declarations the packages' types refer to. */
const KNOWN = [space, ...Object.values(principal)];

/** Create a titled object of a type in the space. */
function create(type: ObjectType, title: string) {
    return type.call("create", { ...type.in(spaceId), title });
}

/** Open the isolate of an installation in a world, whose types have no hooks. */
async function unhooked(
    owner: PackageVersion,
    id: Identifier<"installation">,
    declarations: readonly Declaration[],
    environment: Environment,
): Promise<Isolate> {
    return new Isolate({
        server: { ...(await testServerContext(owner, spaceId, environment)), installation: id },
        declarations,
        send: () => Promise.reject(new TypeError("the installation has no hooks")),
    });
}

/** Describe a package's one type and its handle routing it, as its build's declarations do. */
function declarationsOf(owner: PackageVersion, type: ObjectType, route: string): Declaration[] {
    // describe the type as the package's own, and its handle routing it
    const description = { ...ObjectDescription.of(type), package: owner };
    const own = nth(ObjectTypePool.of(KNOWN).declare([description]), 0);
    const handle = Package.of(owner, { objects: [own] });
    const symbol = graph.Moniker.of({ packageId: owner.id, module: "src/index.ts", name: route });

    return [
        { symbol, ...OBJECT_KIND.identity, description },
        {
            symbol,
            ...PACKAGE_KIND.identity,
            description: describePackage({ ...handle, ...testIdentity }),
        },
    ];
}

test("serve two installed packages from one store, writing both in one mutation or neither", async () => {
    // serve the notebooks' and the tasks' isolated types from one store over the space's database
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { space }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const isolates = [
        await unhooked(
            notebooks,
            schema
                .identifier("installation")
                .parse("installation-01996ab0-0000-7000-8000-0000000000c1"),
            declarationsOf(notebooks, named("notebook", "notebooks"), "notebook"),
            world,
        ),
        await unhooked(
            tasks,
            schema
                .identifier("installation")
                .parse("installation-01996ab0-0000-7000-8000-0000000000c2"),
            declarationsOf(tasks, named("task", "tasks"), "task"),
            world,
        ),
    ];
    const store = new ObjectStore({
        servers: [],
        chain: async () => undefined,
        environment: world,
        blobs: new MemoryBlobStorage(world.crypto),
        principal: spacePrincipal(spaceId),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: notebooks },
        policies: [space],
        isolates,
    });

    // migrate both packages' tables as the space's main database resource does
    const types = store.objects.filter((type) =>
        [notebooks.id, tasks.id].some((id) => id === type.package.id),
    );
    const tables = types.flatMap((type) => type.tables);
    await storage.database.migrate(tables, {
        manager: MAIN_DATABASE,
        packages: [notebooks.id, tasks.id],
    });
    const [notebook, task] = [nth(types, 0), nth(types, 1)];
    const caller = store.as(notebook);

    // write a notebook and a task in one mutation
    await store.mutate(caller, {
        id: RequestId.create(world),
        calls: [create(notebook, "Work"), create(task, "Plan the week")],
    });

    // write neither when the task's call fails
    const refused = await store
        .mutate(caller, {
            id: RequestId.create(world),
            calls: [create(notebook, "Home"), create(task, "")],
        })
        .then(
            () => "written",
            () => "refused",
        );

    // read both packages' objects through the one store
    const list = async (type: ObjectType) =>
        Listed.parse(await store.query(caller, type.call("list", type.in(spaceId)))).items.map(
            (item) => item.title,
        );
    expect({
        refused,
        notebooks: await list(notebook),
        tasks: await list(task),
        tables: types.map((type) => type.table[TABLE].sqlName),
    }).toEqual({
        refused: "refused",
        notebooks: ["Work"],
        tasks: ["Plan the week"],
        tables: ["example__notebooks__notebook", "example__tasks__task"],
    });
});
