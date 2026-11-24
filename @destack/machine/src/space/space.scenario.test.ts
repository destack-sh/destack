import { installation, database, installationRevision } from "@destack/stack/object";
import { testIdentity } from "@destack/object/test";
import { eq } from "@destack/db";
import { testBuild } from "@destack/object/test";
import { expect, refusal, test } from "@destack/test";
import { Digest, Identifier, nth, present, schema } from "@destack/schema";
import { type Capability, Package } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import type { ServerContext } from "@destack/object/workload";
import { defineKind, PackageId, PackageVersion } from "@destack/package";
import { BuildReader, BuildListCache } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { testHost } from "@destack/test/host";
import { STACKS } from "@destack/account/test";
import objectDefinition from "@destack/object/destack.json" with { type: "json" };
import { describeObject } from "@destack/object/inspect";
import { ids, insertRelease, insertInstallation, readStack, TestSpace } from "../test/index.ts";
import { note } from "../test/schema.ts";
import { t } from "@destack/locale";

/** The one host the test package lists by name. */
const DECLARED_HOST = "api.example.com";

/** Install a package calling some hosts in a test space, answering the context it serves with. */
async function installCaller(connect: readonly string[]) {
    // ship a server declaring network access to the hosts, keeping the context its installation serves with
    const released = PackageVersion.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000e2",
        name: "@example/caller",
        version: "2026.9.0",
    });
    const call: Capability = {
        reason: t`Call the example API`,
        sandbox: { network: { connect: [...connect] } },
    };
    const handle = {
        ...Package.of(released, { objects: [], capabilities: [{ name: "call", ...call }] }),
        ...testIdentity,
    };
    const build = await testBuild(handle, testHost().crypto);
    const contexts: ServerContext[] = [];
    const fixture = await TestSpace.open({
        servers: [
            implementPackage(handle, (context) => {
                contexts.push(context);

                return { objects: [] };
            }),
        ],
        linked: [build],
    });
    await fixture.install(build);
    await fixture.serve();

    return { fixture, context: present(contexts.at(-1), "the installation's context") };
}

test("refuse an in-process installation's calls to hosts its capabilities omit", async () => {
    const { context } = await installCaller([DECLARED_HOST]);

    // call an undeclared host and open a socket to another
    expect([
        await refusal(context.fetch(new Request("https://other.example.com/"))),
        await refusal(
            Promise.resolve().then(() =>
                context.environment.network.connect({ hostname: "smtp.example.com", port: 25 }),
            ),
        ),
    ]).toEqual([
        [
            "FORBIDDEN",
            "NETWORK_DENIED",
            "network access to 'other.example.com:443' is forbidden: installation 'caller' declares no access to it",
        ],
        [
            "FORBIDDEN",
            "NETWORK_DENIED",
            "network access to 'smtp.example.com:25' is forbidden: installation 'caller' declares no access to it",
        ],
    ]);
});

test("follow a wildcard installation's redirects without credentials, refusing replays and private hosts", async () => {
    // serve a host redirecting to another origin, which records the headers it receives
    const { fixture, context } = await installCaller(["*", DECLARED_HOST]);
    const received: (string | null)[] = [];
    const network = fixture.simulation.network;
    network.server(DECLARED_HOST).http({
        hostname: DECLARED_HOST,
        port: 443,
        fetch: (request) =>
            new Response(null, {
                status: new URL(request.url).pathname === "/post" ? 307 : 302,
                headers: { location: "https://files.example.org/asset" },
            }),
    });
    network.server("files.example.org").http({
        hostname: "files.example.org",
        port: 443,
        fetch: (request) => {
            received.push(request.headers.get("authorization"));

            return new Response("asset");
        },
    });

    // fetch through the redirect with a credential, then refuse a refused and a body-replaying redirect and a private host
    const followed = await context.fetch(
        new Request(`https://${DECLARED_HOST}/asset`, { headers: { authorization: "Bearer key" } }),
    );
    expect([
        followed.status,
        await followed.text(),
        received,
        await refusal(
            context.fetch(new Request(`https://${DECLARED_HOST}/asset`, { redirect: "error" })),
        ),
        await refusal(
            context.fetch(
                new Request(`https://${DECLARED_HOST}/post`, { method: "POST", body: "{}" }),
            ),
        ),
        await refusal(context.fetch(new Request("http://localhost:8080/"))),
    ]).toEqual([
        200,
        "asset",
        [null],
        [
            "BAD_GATEWAY",
            "REDIRECT_UNREACHABLE",
            `could not fetch 'https://${DECLARED_HOST}/asset': it redirected and the request refuses redirects`,
        ],
        [
            "BAD_GATEWAY",
            "REDIRECT_UNREACHABLE",
            `could not follow the 307 redirect of 'https://${DECLARED_HOST}/post': it would send the body again`,
        ],
        [
            "FORBIDDEN",
            "ADDRESS_PRIVATE_DENIED",
            "host 'localhost' is forbidden: it resolves to 'localhost', outside the public network",
        ],
    ]);
});

/** The tables the notes installation asks main for. */
const STATE = { tables: { sqlite: [], postgresql: [] } };

test("serve an installation only once it is bound and main applied its tables", async () => {
    // install notes at a revision declaring a note type, bound to main, on a stopped machine so no job reads beside
    const fixture = await TestSpace.open();
    const { database: connection } = fixture;
    const space = await fixture.space;
    const version = await fixture.releaseRow(ids.package, "2026.9.0");
    await fixture.stop();
    const now = fixture.machine.clock.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const main = nth(await connection.select().from(database.table), 0);
    const stack = await readStack(connection);
    await insertInstallation(
        connection,
        {
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            version,
            installer: fixture.owner.subject,
        },
        record.createdAt,
    );
    const revision = Identifier.create("installation-revision", fixture.environment);
    const release = {
        package: ids.package,
        version: "2026.9.0",
        image: Digest.parse("d".repeat(64)),
    };
    const pulled = await insertRelease(
        connection,
        ids.space,
        release,
        [],
        fixture.machine.environment,
    );
    const state = await space.blobs.writeJson(STATE);
    const declarations = await space.blobs.writeJson([]);
    await connection.transaction(
        async (transaction) =>
            await transaction.insert(installationRevision.table).values({
                ...record,
                id: revision,
                installationId: ids.notes,
                buildId: pulled.id,
                dependencies: [],
                states: { main: state },
                release: release.version,
                declarations,
                generation: 1,
                bindings: [],
                digest: Digest.parse("e".repeat(64)),
            }),
    );
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ targetRevisionId: revision })
                .where(eq(installation.table.id, ids.notes)),
    );
    const notes = await MemoryBuild.of(
        PackageVersion.parse({ id: ids.package, name: "@example/notes", version: "2026.9.0" }),
        [
            {
                kind: "object",
                kindPackageId: objectDefinition.id,
                name: "note",
                description: describeObject(note),
            },
        ],
        fixture.environment.crypto,
    );
    const open = async (packageId: PackageId, image: Digest) =>
        image === release.image ? notes : space.root(packageId, image);
    const served = async (
        opening: (packageId: PackageId, image: Digest) => Promise<BuildReader> = open,
    ) =>
        (await space.installations(opening))
            .map((entry) => [entry.installation.id, entry.declarations.map((each) => each.name)])
            .toSorted(([left], [right]) => String(left).localeCompare(String(right)));

    // open the stack's build with a vault it declares for its document, which nothing binds to it
    const vaulted = async (packageId: PackageId, image: Digest) => {
        const built = await open(packageId, image);

        return packageId === stack.packageId
            ? new VaultedReader(
                  built.manifest,
                  built.stream,
                  fixture.environment.crypto,
                  new BuildListCache(),
              )
            : built;
    };

    // serve nothing while the notes' targets are not bound, though main applied their tables
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(database.table)
                .set({
                    states: [await Digest.json(STATE, connection.state.environment.crypto)],
                    appliedStates: [await Digest.json(STATE, connection.state.environment.crypto)],
                })
                .where(eq(database.table.id, main.id)),
    );
    const unbound = await served();

    // serve nothing while main has not applied the notes' tables, their targets bound
    const isBound = {
        status: "true" as const,
        reason: "Bound",
        message: "",
        observedGeneration: 1,
        lastTransitionAt: now,
    };
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ conditions: { Bound: isBound } })
                .where(eq(installation.table.id, ids.notes)),
    );
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(database.table)
                .set({ appliedStates: [] })
                .where(eq(database.table.id, main.id)),
    );
    const unapplied = await served();

    // serve notes once main applied them
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(database.table)
                .set({
                    appliedStates: [await Digest.json(STATE, connection.state.environment.crypto)],
                })
                .where(eq(database.table.id, main.id)),
    );
    const applied = await served();

    // serve notes while a plan for another state waits for its approval
    await connection.transaction(
        async (transaction) =>
            await transaction
                .update(database.table)
                .set({
                    states: [
                        await Digest.json(STATE, connection.state.environment.crypto),
                        Digest.parse("f".repeat(64)),
                    ],
                })
                .where(eq(database.table.id, main.id)),
    );
    const waiting = await served();
    const withVault = await served(vaulted);

    // serve nothing in a space without main
    await connection.transaction(
        async (transaction) =>
            await transaction.delete(database.table).where(eq(database.table.id, main.id)),
    );
    const unborn = await served();

    // serve the space's stack throughout, its document's vault unbound, and notes once bound and applied
    const stacked = [stack.id, [STACKS.space.export]];
    expect({ unbound, unapplied, applied, waiting, withVault, unborn }).toEqual({
        unbound: [stacked],
        unapplied: [stacked],
        applied: [[ids.notes, ["note"]], stacked],
        waiting: [[ids.notes, ["note"]], stacked],
        withVault: [
            [ids.notes, ["note"]],
            [stack.id, [STACKS.space.export, "credentials"]],
        ],
        unborn: [],
    });
});

/** Vaults, which this test's builds declare without a provider. */
const VaultKind = defineKind(
    { name: "vault", provisioned: { spec: schema.object({}) } },
    { package: note.package },
);

/** A build that declares a vault besides its own declarations. */
class VaultedReader extends BuildReader {
    /** Read the build's declarations with a vault named credentials. */
    override async declarations() {
        const declarations = await super.declarations();
        const vault = {
            ...VaultKind.identity,
            provisioned: true as const,
            name: "credentials",
            description: { name: "credentials", spec: {} },
        };

        return [...declarations, { ...nth(declarations, 0), ...vault }];
    }
}
