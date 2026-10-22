import { STACKS } from "@destack/account/test";
import { PackageHandle } from "@destack/object";
import { Health, implementPackage, ObjectStore, Server } from "@destack/object/server";
import { testEvents, serversOf, subjectContext, testCallKey } from "@destack/object/test";
import { onTestFinished, refusal, test } from "@destack/test";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { AccessContext, Authorization, Authorizer, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "@destack/object/client";
import { graph, PackageId } from "@destack/package";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { ResourceContext } from "@destack/resource/context";
import { Digest, found, present, schema } from "@destack/schema";
import { BunGraphStorage } from "../src/bun/index.ts";
import { build, CodeIndex, declaration, module, symbol } from "../src/object/index.ts";
import { stackTables } from "../src/stack/index.ts";
import { ids } from "../src/test/index.ts";
import { Clock } from "@destack/environment";

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

/** The build the test reads. */
const buildId = schema.identifier("build").parse("build-01996ab0-0000-7000-8000-000000000003");

/** A second build of the same commit, which shares the first one's index. */
const rebuiltId = schema.identifier("build").parse("build-01996ab0-0000-7000-8000-000000000004");

/** The repository the build compiles. */
const repositoryId = schema
    .identifier("repository")
    .parse("repository-01996ab0-0000-7000-8000-000000000005");

test("serve a build's modules, declarations and code symbols from its index to the build's readers, through the object store and a client", async ({
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
    const code = notebookCode();
    const codeBytes = new TextEncoder().encode(JSON.stringify(code));
    const codeFile = {
        bytes: codeBytes,
        digest: await Digest.of(codeBytes),
        size: codeBytes.byteLength,
    };

    // record it as two of the space's succeeded builds of one commit, the second finished later
    const record = async (id: typeof buildId, finishedAt: number) => {
        await database.insert(build.table).values({
            id,
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
            finishedAt,
            createdAt: now,
            updatedAt: now,
        });
        await Authorization.run(
            authorizer,
            database,
            () => AccessContext.of(Caller.of(owner), now),
            (authorization) => authorization.create(build.reference(ids.space, id), {}),
        );
    };
    await record(buildId, now);
    await record(rebuiltId, now + 1);

    // read the build's graph through the object store as its owner and refuse a stranger
    await using graphs = new BunGraphStorage(
        database,
        {
            contains: async () => true,
            contents: async (read) => {
                expect(read).toEqual(manifest);

                return stored;
            },
            referrer: async (read, path) => {
                expect([read, path]).toEqual([manifest, "code.json"]);

                return CodeIndex.file({ digest: codeFile.digest, size: codeFile.size });
            },
            file: async () => codeFile.bytes,
        },
        join(directory, "graph"),
    );
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: principal.space.reference("universe", ids.space),
        servers: serversOf([module, declaration, symbol], { external: graphs }),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: symbol.package },
    });
    const context = (user: string) =>
        subjectContext(principal.user.reference("universe", user), buildId);
    const moniker = (path: string) => `${NOTEBOOK_ID}/${path}`;
    const at = moniker("src/object.ts");
    const declared = async (id: string) => {
        const row = await store.query(
            context(ids.owner).access(),
            declaration.call("get", { buildId, id }),
        );

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
        const row = await store.query(
            context(ids.owner).access(),
            symbol.call("get", { buildId, id }),
        );

        return [
            row.id,
            row.moduleId,
            row.kind,
            row.name,
            row.signature,
            relations(row.relationships),
        ];
    };
    const modules = await store.query(
        context(ids.owner).access(),
        module.call("list", { buildId }),
    );

    // read the rebuild's notebook variable, in its own scope and finish, from the one index
    const rebuilt = await store.query(
        context(ids.owner).access(),
        symbol.call("get", { buildId: rebuiltId, id: `${at}#notebook` }),
    );

    // list every module with its dependencies and the notebook's object declaration at its variable
    expect([
        modules.items.map((row) => [row.id, row.path, relations(row.relationships)]),
        await declared(`${at}#notebook:object:notebook`),
        await coded(`${at}#notebook`),
        [rebuilt.scope, rebuilt.createdAt],
        await readdir(join(directory, "graph")),
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
        [rebuiltId, now + 1],
        [`${manifest}.sqlite`],
    ]);
    const refused = await refusal(
        store.query(
            context("stranger").access(),
            symbol.call("get", { buildId, id: `${at}#notebook` }),
        ),
    );
    expect(refused).toEqual(["NOT_FOUND", `no scope ${buildId}`]);

    // serve the graph objects to a client
    const indexed = PackageHandle.of(symbol.package, { objects: { module, declaration } });
    const served = Server.start({
        clock: Clock.system,
        ...new ObjectStore({
            servers: [
                implementPackage(indexed, { objects: [module, declaration], external: graphs }),
            ],
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
    const local = await TestDatabase.create("sqlite", ObjectClient.tables({ module, declaration }));
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        clock: Clock.system,
        database: local.database,
        objects: { module, declaration },
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
    const procedures = client.query.declaration
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
        {
            contains: async () => false,
            contents: () => Promise.reject(new TypeError("the test stores no builds")),
            referrer: () => Promise.reject(new TypeError("the test stores no builds")),
            file: () => Promise.reject(new TypeError("the test stores no builds")),
        },
        join(directory, "graph"),
    );

    // sweep while the open fails and refuse the open alone
    const opening = graphs.open(buildId);
    const swept = graphs.sweep();
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
            digest: "0".repeat(64),
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
    const served = graph.Declaration.at(service, {
        kind: "service",
        kindPackageId: PackageId.parse(packages.object),
        name: "notebooks",
        description: {},
    });
    const procedure = (name: string) =>
        graph.Declaration.member(served, { kind: "procedure", name, description: {} });

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

    return MemoryBuild.write(sources, {
        package: {
            id: PackageId.parse(NOTEBOOK_ID),
            name: "@example/notebook",
            version: "2026.10.0",
        },
        graph: written,
    });
}
