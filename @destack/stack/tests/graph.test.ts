import { testHost } from "@destack/test/host";
import { AccessContext, Caller, principal } from "@destack/access";
import { eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/object/test";
import { ObjectClient } from "@destack/object/client";
import { graph, PackageId } from "@destack/package";
import { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Digest, found, schema } from "@destack/schema";
import { Scope, type Subject } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { BunGraphStorage } from "../src/bun/index.ts";
import { build, declaration, module, symbol } from "../src/object/index.ts";
import stackPackage from "../src/package.ts";
import { spaceTables } from "../src/stack/index.ts";
import { TestSpace } from "../src/test/index.ts";

/** The notebook's package identifier. */
const NOTEBOOK_ID = "package-01996ab0-0000-7000-8000-00000000000a";

/** The package identifiers of the packages the notebook depends on. */
const packages = {
    account: "package-01a0c80b-614c-76c1-90c6-f83bdcd72b32",
    db: "package-01a0c80b-614f-73f4-a451-d79e4b96b4d1",
    object: "package-01a0d462-1b26-7401-be27-6258e0a32141",
    package: "package-01a0c80b-614d-75ab-8b3b-456b8fdb73f1",
    schema: "package-01a0c80b-614e-739b-bf41-8570d1151a11",
};

/** A person with no role in the space. */
const STRANGER = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000006");

/** A build no store has. */
const UNKNOWN_BUILD = schema
    .identifier("build")
    .parse("build-01996ab0-0000-7000-8000-000000000003");

test("serve a build's modules, declarations and code symbols from its index to the build's readers, through the object store and a client", async () => {
    // record the notebook's build with its code index twice as the space's builds of one manifest
    const fixture = await TestSpace.open();
    const { simulation } = fixture;
    const { clock } = simulation;
    clock.resume();
    const served = await fixture.serve();
    const notebook = await writeNotebook();
    const stored = notebook.reader;
    const index = { code: notebookCode(), modules: notebook.modules };
    const { build: buildId } = await fixture.recordBuild(stored, index);
    clock.advance(1);
    const { build: rebuiltId } = await fixture.recordBuild(stored, index);
    const [rebuild] = await fixture.database
        .select({ finishedAt: build.table.finishedAt })
        .from(build.table)
        .where(eq(build.table.id, rebuiltId));

    // read the build's graph through the space's store as its owner and refuse a stranger
    const reader = (user: Subject) => AccessContext.of(Caller.of(user), clock.now());
    const owner = reader(fixture.owner.subject);
    const moniker = (path: string) => `${NOTEBOOK_ID}/${path}`;
    const at = moniker("src/object.ts");
    const { store } = served;
    const declared = async (id: string) => {
        const row = await store.query(owner, declaration.call("get", { buildId, id }));

        return [
            row.id,
            row.moduleId,
            row.kind,
            row.name,
            row.enclosing,
            relations(row.relationships),
        ];
    };
    const coded = async (id: string) => {
        const row = await store.query(owner, symbol.call("get", { buildId, id }));

        return [
            row.id,
            row.moduleId,
            row.kind,
            row.name,
            row.signature,
            relations(row.relationships),
        ];
    };
    const modules = await store.query(owner, module.call("list", { buildId }));

    // read the rebuild's notebook variable, in its own scope and finish, from the one index
    const rebuilt = await store.query(
        owner,
        symbol.call("get", { buildId: rebuiltId, id: `${at}#notebook` }),
    );
    const stranger = reader(principal.user.reference(Scope.universe.id, STRANGER));
    const refused = await refusal(
        store.query(stranger, symbol.call("get", { buildId, id: `${at}#notebook` })),
    );

    // list every module with its dependencies and the notebook's object declaration at its variable
    expect([
        modules.items.map((row) => [row.id, row.path, relations(row.relationships)]),
        await declared(`${at}#notebook:object:notebook`),
        await coded(`${at}#notebook`),
        [rebuilt.scope, rebuilt.createdAt],
        refused,
    ]).toEqual([
        [
            [moniker("src/index.ts"), "src/index.ts", ["depends src/object.ts"]],
            [at, "src/object.ts", [`depends ${packages.object}`]],
            [moniker("src/service.ts"), "src/service.ts", ["depends src/object.ts"]],
        ],
        [`${at}#notebook:object:notebook`, at, "object", "notebook", `${at}#notebook`, []],
        [
            `${at}#notebook`,
            at,
            "variable",
            "notebook",
            NOTEBOOK_SIGNATURE,
            [`calls ${packages.object}`],
        ],
        [rebuiltId, rebuild?.finishedAt],
        ["NOT_FOUND", "NOT_FOUND", `symbol ${at}#notebook not found`],
    ]);

    // read the service's procedures in the owner's client's live query
    const browser = simulation.environment("reader");
    const local = await TestDatabase.create(ObjectClient.tables({ module, declaration }), {
        environment: browser,
        dialect: "sqlite",
    });
    onTestFinished(() => local.close());
    const endpoint = served.as(fixture.owner.subject).endpoint(stackPackage);
    const client = await ObjectClient.open({
        environment: browser,
        database: local.database,
        objects: { module, declaration },
        scope: buildId,
        caller: fixture.owner.subject,
        endpoint,
        reconnect: (holder) => {
            throw new TypeError(`no build moves, yet one moved to ${holder}`);
        },
    });
    const stopping = new AbortController();
    const failures: unknown[] = [];
    const running = client.run(stopping.signal, (error) => failures.push(error));
    onTestFinished(async () => {
        stopping.abort();
        await running;
        await client.close();
    });
    const service = `${moniker("src/service.ts")}#notebooksService`;
    const procedures = client.query.declaration
        .findMany({ where: { kind: "procedure" }, orderBy: { id: "asc" } })
        .subscribe();
    await procedures.ready;
    expect([(await procedures.read()).map((row) => [row.enclosing, row.name]), failures]).toEqual([
        [
            [service, "notebook.get"],
            [service, "notebook.list"],
        ],
        [],
    ]);
});

test("refuse to open the graph of a build the space does not record", async () => {
    // add an empty space database in the test's world
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(spaceTables({}).tables, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await using graphs = new BunGraphStorage(
        storage.database,
        {
            store: refuse,
            contains: async () => false,
            contents: refuse,
            referrer: refuse,
            file: refuse,
        },
        world.files,
    );

    // refuse the open of an unknown build
    expect(await graphs.open(UNKNOWN_BUILD).catch((error: unknown) => error)).toEqual(
        new TypeError(`build ${UNKNOWN_BUILD} is missing`),
    );
});

/** Refuse a read of a build, as a store with no builds does. */
function refuse(): Promise<never> {
    return Promise.reject(new TypeError("the test stores no builds"));
}

/** List relationships once each as kind and target, by path in the fixture, else by package. */
function relations(
    relationships: readonly { readonly kind: string; readonly to: string }[],
): string[] {
    const listed = relationships.map(({ kind, to }) => {
        const target = to.startsWith(`${NOTEBOOK_ID}/`)
            ? to.slice(NOTEBOOK_ID.length + 1)
            : (/^(npm:(?:@[^/]+\/)?[^/]+|package-[^/]+)\//u.exec(to)?.[1] ?? to);

        return `${kind} ${target}`;
    });

    return [...new Set(listed)];
}

/** The printed declaration of the notebook's variable. */
const NOTEBOOK_SIGNATURE = "const notebook: ObjectType";

/** The notebook's code index: its variable, which calls the object package's constructor. */
function notebookCode(): graph.Document[] {
    const notebook = graph.Moniker.of({
        packageId: NOTEBOOK_ID,
        module: "src/object.ts",
        name: "notebook",
    });

    return [
        {
            path: "src/object.ts",
            digest: Digest.parse("0".repeat(64)),
            symbols: [
                {
                    moniker: notebook,
                    kind: "variable",
                    source: { file: "src/object.ts", start: 0, end: 20 },
                    signature: NOTEBOOK_SIGNATURE,
                    isExported: true,
                },
            ],
            edges: [
                {
                    from: notebook,
                    to: graph.Moniker.parse(`${packages.object}/src/object.ts#defineObject`),
                    kind: "calls",
                },
            ],
        },
    ];
}

/** Write the notebook's build and its module graph: an entry, the object and two procedures. */
async function writeNotebook(): Promise<{ reader: BuildReader; modules: graph.Module[] }> {
    // name each module's symbols and declarations by moniker
    const monikerOf = (path: string, name?: string) =>
        graph.Moniker.of({
            packageId: NOTEBOOK_ID,
            module: path,
            ...(name === undefined ? {} : { name }),
        });
    const notebook = monikerOf("src/object.ts", "notebook");
    const service = monikerOf("src/service.ts", "notebooksService");
    const served = graph.Declaration.at(service, {
        kind: "service",
        kindPackageId: PackageId.parse(packages.object),
        name: "notebooks",
        description: {},
    });
    const procedure = (name: string) =>
        graph.Declaration.member(served, { kind: "procedure", name, description: {} });

    // write each module's source next to its graph file
    const sources = new Map(
        ["src/index.ts", "src/object.ts", "src/service.ts"].map((path) => [
            path,
            new TextEncoder().encode(`// ${path}\n`),
        ]),
    );
    const digest = (path: string) => Digest.of(found(sources, path), testHost().crypto);
    const written: graph.Module[] = [
        {
            path: "src/index.ts",
            digest: await digest("src/index.ts"),
            imports: ["./object.ts"],
            exports: [],
            declarations: [],
            edges: [
                {
                    from: monikerOf("src/index.ts"),
                    to: monikerOf("src/object.ts"),
                    kind: "depends",
                },
            ],
        },
        {
            path: "src/object.ts",
            digest: await digest("src/object.ts"),
            imports: ["@destack/object"],
            exports: [{ name: "notebook", symbol: notebook, isTypeOnly: false }],
            declarations: [
                graph.Declaration.at(notebook, {
                    kind: "object",
                    kindPackageId: PackageId.parse(packages.object),
                    name: "notebook",
                    description: {},
                }),
            ],
            edges: [
                {
                    from: monikerOf("src/object.ts"),
                    to: graph.Moniker.parse(`${packages.object}/src/index.ts`),
                    kind: "depends",
                },
            ],
        },
        {
            path: "src/service.ts",
            digest: await digest("src/service.ts"),
            imports: ["./object.ts"],
            exports: [{ name: "notebooksService", symbol: service, isTypeOnly: false }],
            declarations: [served, procedure("notebook.get"), procedure("notebook.list")],
            edges: [
                {
                    from: monikerOf("src/service.ts"),
                    to: monikerOf("src/object.ts"),
                    kind: "depends",
                },
            ],
        },
    ];

    const reader = await MemoryBuild.write(
        sources,
        {
            package: {
                id: PackageId.parse(NOTEBOOK_ID),
                name: "@example/notebook",
                version: "2026.10.0",
            },
        },
        testHost().crypto,
    );

    return { reader, modules: written };
}
