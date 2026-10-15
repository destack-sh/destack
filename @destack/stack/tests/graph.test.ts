import { STACKS } from "@destack/account/test";
import { PackageHandle } from "@destack/runtime";
import { testEvents, serversOf } from "@destack/object/test";
import { onTestFinished, refusal, test } from "@destack/test";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { AccessContext, Authorization, Authorizer, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "@destack/object/client";
import { defineServer, ObjectStore } from "@destack/object/server";
import { graph, PackageId } from "@destack/package";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { ResourceContext } from "@destack/resource/context";
import { Clock, Digest, found, present, schema } from "@destack/schema";
import { Health } from "@destack/runtime/health";
import { Server } from "@destack/runtime/server";
import { subjectContext, testCallKey } from "@destack/runtime/test";
import { BunGraphStorage } from "../src/bun/index.ts";
import { module, symbol } from "../src/graph/index.ts";
import { build } from "../src/object/index.ts";
import { stackTables } from "../src/stack/index.ts";
import { ids } from "../src/test/index.ts";

/** The notebook's package identifier. */
const NOTEBOOK_ID = "package-01996ab0-0000-7000-8000-00000000000a";

/** The package identifiers of the packages the notebook depends on. */
const packages = {
    account: "package-01a0c80b-614c-76c1-90c6-f83bdcd72b32",
    audit: "package-01a0c80b-614e-739d-8c3c-55f69a40525a",
    db: "package-01a0c80b-614f-73f4-a451-d79e4b96b4d1",
    object: "package-01a0d462-1b26-7401-be27-6258e0a32141",
    package: "package-01a0c80b-614d-75ab-8b3b-456b8fdb73f1",
    schema: "package-01a0c80b-614e-739b-bf41-8570d1151a11",
    service: "package-01a0c80b-614f-73ee-8687-f9c041e21f17",
};

/** The build the test reads. */
const buildId = schema.identifier("build").parse("build-01996ab0-0000-7000-8000-000000000003");

/** The repository the build compiles. */
const repositoryId = schema
    .identifier("repository")
    .parse("repository-01996ab0-0000-7000-8000-000000000005");

test("serve a build's modules and symbols from its index to the build's readers, through the object store and a client", async ({
    expect,
}) => {
    // keep an owned space, its row replicated beside the space's tables
    const storage = await TestDatabase.create("sqlite", [...stackTables, space.table], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const now = Date.now();
    const owner = principal.user.reference("universe", ids.owner);
    await new AccessFixture(database).replicateScope(account.reference("universe", ids.account));
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        stack: STACKS.space,
        createdAt: now,
        updatedAt: now,
    });
    const authorizer = new Authorizer([space.policy, build.policy], [space.mapping, build.mapping]);
    await Authorization.run(
        authorizer,
        database,
        () => AccessContext.of(Caller.of(owner), now),
        (authorization) => authorization.create(space.reference(ids.account, ids.space), { owner }),
    );

    // write the notebook's build with its graph: an entry, an object and a service with two procedures
    const directory = await mkdtemp(join(tmpdir(), "destack-graph-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    const stored = await writeNotebook();
    const manifest = await Digest.of(
        new TextEncoder().encode(JSON.stringify(PackageManifest.parse(stored.manifest))),
    );

    // record it as the space's succeeded build
    await database.insert(build.table).values({
        id: buildId,
        scope: ids.space,
        source: {
            kind: "commit",
            repository: repositoryId,
            commit: "a".repeat(40),
            directory: ".",
        },
        outputs: ["bun"],
        state: "succeeded",
        manifest,
        startedAt: now,
        finishedAt: now,
        createdAt: now,
        updatedAt: now,
    });
    await Authorization.run(
        authorizer,
        database,
        () => AccessContext.of(Caller.of(owner), now),
        (authorization) => authorization.create(build.reference(ids.space, buildId), {}),
    );

    // read the build's graph through the object store as its owner and refuse a stranger
    await using graphs = new BunGraphStorage(
        database,
        async (spaceId, read) => {
            expect([spaceId, read]).toEqual([ids.space, manifest]);

            return stored;
        },
        join(directory, "graph"),
    );
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: principal.space.reference("universe", ids.space),
        servers: serversOf([module, symbol], { external: graphs }),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: symbol.package },
    });
    const context = (user: string) =>
        subjectContext(principal.user.reference("universe", user), buildId);
    const moniker = (path: string) => `${NOTEBOOK_ID}/${path}`;
    const at = moniker("src/object.ts");
    const read = (id: string) =>
        store.query(context(ids.owner).access(), symbol.call("get", { buildId, id }));
    const summary = (row: Awaited<ReturnType<typeof read>>) => [
        row.id,
        row.moduleId,
        row.kind,
        row.name,
        row.enclosing,
        relations(row.relationships),
    ];
    const modules = await store.query(
        context(ids.owner).access(),
        module.call("list", { buildId }),
    );

    // list every module with its dependencies and the notebook's object declaration at its variable
    expect([
        modules.items.map((row) => [row.id, row.path, relations(row.relationships)]),
        summary(await read(`${at}#notebook:object`)),
        summary(await read(`${at}#notebook`)),
        await readdir(join(directory, "graph", manifest)),
    ]).toEqual([
        [
            [moniker("src/index.ts"), "src/index.ts", ["depends src/object.ts"]],
            [at, "src/object.ts", [`depends ${packages.object}`]],
            [moniker("src/service.ts"), "src/service.ts", ["depends src/object.ts"]],
        ],
        [`${at}#notebook:object`, at, "object", "notebook", `${at}#notebook`, []],
        [`${at}#notebook`, at, "variable", "notebook", null, [`calls ${packages.object}`]],
        [`${buildId}.sqlite`],
    ]);
    const refused = await refusal(
        store.query(
            context("stranger").access(),
            symbol.call("get", { buildId, id: `${at}#notebook` }),
        ),
    );
    expect(refused).toEqual(["NOT_FOUND", `no scope ${buildId}`]);

    // serve the graph objects to a client
    const indexed = PackageHandle.of(symbol.package, { objects: { module, symbol } });
    const served = Server.start({
        clock: Clock.system,
        ...new ObjectStore({
            servers: [defineServer(indexed, { objects: [module, symbol], external: graphs })],
            chain: async () => undefined,
            clock: Clock.system,
            origin: { package: indexed },
            principal: principal.space.reference("universe", ids.space),
            database,
            callKey: testCallKey,
            events: await testEvents(database),
        }).package(indexed),
        scope: buildId,
        resources: new ResourceContext(),
        health: new Health("graph"),
        drainTimeout: 1000,
        authorizeMachine: async () => {},
        authenticate: async (request) => {
            const user = present(request.headers.get("authorization"), "the authorization header");
            const subject = principal.user.reference("universe", user);

            return {
                caller: Caller.of(subject),
                credential: {
                    kind: "session",
                    id: user,
                    audience: indexed.id,
                    scope: buildId,
                    verifiedAt: now,
                    expiresAt: now + 60_000,
                },
            };
        },
    });
    onTestFinished(() => served.close());
    const local = await TestDatabase.create("sqlite", ObjectClient.tables({ module, symbol }));
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        clock: Clock.system,
        database: local.database,
        objects: { module, symbol },
        scope: buildId,
        caller: owner,
        endpoint: {
            url: "https://graph.test",
            headers: { authorization: ids.owner },
            fetch: (request: Request) => served.fetch(request),
        },
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

    // read the service's procedures in the client's live query
    const service = `${moniker("src/service.ts")}#notebooksService`;
    const procedures = client.query.symbol
        .findMany({ where: { kind: "procedure" }, orderBy: { id: "asc" } })
        .subscribe();
    await procedures.ready;
    expect((await procedures.read()).map((row) => [row.enclosing, row.name])).toEqual([
        [service, "notebook.get"],
        [service, "notebook.list"],
    ]);
    expect(failures).toEqual([]);
});

test("sweep the indexes retention drops while an open of an unknown build fails", async ({
    expect,
}) => {
    // keep an empty space database and a graph directory
    const storage = await TestDatabase.create("sqlite", stackTables, { isMigrated: true });
    onTestFinished(() => storage.close());
    const directory = await mkdtemp(join(tmpdir(), "destack-graph-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    await using graphs = new BunGraphStorage(
        storage.database,
        () => Promise.reject(new TypeError("the test stores no builds")),
        join(directory, "graph"),
    );

    // sweep while the open fails and refuse the open alone
    const opening = graphs.open(buildId);
    const swept = graphs.sweep(new Set());
    expect(await Promise.allSettled([swept, opening])).toEqual([
        { status: "fulfilled", value: undefined },
        { status: "rejected", reason: new TypeError(`build ${buildId} is missing`) },
    ]);
});

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

/** Declare an exported variable of a module at its moniker. */
function variable(moniker: graph.Moniker, path: string) {
    return {
        moniker,
        kind: "variable" as const,
        source: { file: path, start: 0, end: 40 },
        signature: "const",
        isExported: true,
    };
}

/** Write the notebook's build: an entry, the notebook object, and two procedures serving it. */
async function writeNotebook(): Promise<BuildReader> {
    // name each module's symbols and declarations by moniker
    const monikerOf = (path: string, name?: string) =>
        graph.Moniker.of({
            packageId: NOTEBOOK_ID,
            module: path,
            ...(name === undefined ? {} : { name }),
        });
    const notebook = monikerOf("src/object.ts", "notebook");
    const service = monikerOf("src/service.ts", "notebooksService");
    const procedure = (name: string) => ({
        moniker: graph.Moniker.of({
            packageId: NOTEBOOK_ID,
            module: "src/service.ts",
            name: "notebooksService",
            member: name,
            kind: "procedure",
        }),
        symbol: service,
        kind: "procedure",
        package: PackageId.parse(packages.service),
        name,
        description: {},
    });

    // write each module's source beside its graph file
    const sources = new Map(
        ["src/index.ts", "src/object.ts", "src/service.ts"].map((path) => [
            path,
            new TextEncoder().encode(`// ${path}\n`),
        ]),
    );
    const digest = (path: string) => Digest.of(found(sources, path));
    const written: graph.Module[] = [
        {
            path: "src/index.ts",
            digest: await digest("src/index.ts"),
            imports: ["./object.ts"],
            exports: [],
            symbols: [],
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
            symbols: [variable(notebook, "src/object.ts")],
            declarations: [
                {
                    moniker: graph.Moniker.parse(`${notebook}:object`),
                    symbol: notebook,
                    kind: "object",
                    package: PackageId.parse(packages.object),
                    name: "notebook",
                    description: {},
                },
            ],
            edges: [
                {
                    from: monikerOf("src/object.ts"),
                    to: graph.Moniker.parse(`${packages.object}/src/index.ts`),
                    kind: "depends",
                },
                {
                    from: notebook,
                    to: graph.Moniker.parse(`${packages.object}/src/object/object.ts#defineObject`),
                    kind: "calls",
                },
            ],
        },
        {
            path: "src/service.ts",
            digest: await digest("src/service.ts"),
            imports: ["./object.ts"],
            exports: [{ name: "notebooksService", symbol: service, isTypeOnly: false }],
            symbols: [variable(service, "src/service.ts")],
            declarations: [procedure("notebook.get"), procedure("notebook.list")],
            edges: [
                {
                    from: monikerOf("src/service.ts"),
                    to: monikerOf("src/object.ts"),
                    kind: "depends",
                },
            ],
        },
    ];

    return MemoryBuild.write(sources, {
        package: {
            id: PackageId.parse(NOTEBOOK_ID),
            name: "@example/notebook",
            version: "2026.10.0",
        },
        graph: written,
    });
}
