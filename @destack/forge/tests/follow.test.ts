import type { Crypto } from "@destack/environment";
import type { Environment } from "@destack/environment";
import { PackageId } from "@destack/package";
import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { reference, referenceShape, repository, tag } from "../src/object/index.ts";
import type { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Commit, Identifier, present, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { build, installationRevision, rollout } from "@destack/stack/object";
import { serveSpace } from "@destack/stack/server";
import stackPackage from "@destack/stack/package";
import objectPackage from "@destack/object/package";
import { ids, reconcile, TestSpace } from "@destack/stack/test";
import { TestEntitlement } from "@destack/account/test";
import { expect, refusal, test } from "@destack/test";
import { Forge } from "../src/client/index.ts";
import { Claim, ClaimLookup, claimTable, Shape } from "@destack/object";

/** The residency's home space, where its forge stores the repositories' references. */
const FORGE_HOME = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000007");

/** Write a built package with one module, as a runner compiles it. */
function writeBuild(crypto: Crypto): Promise<BuildReader> {
    return MemoryBuild.write(
        new Map([["index.js", new TextEncoder().encode("export {};\n")]]),
        {
            package: { id: ids.package, name: "@example/library", version: "2026.9.0" },
        },
        crypto,
    );
}

/** Claim a new repository of an account in the universe's claims. */
async function claimRepository(
    environment: Environment,
    universe: DatabaseConnection,
    accountId: string,
): Promise<Identifier<"repository">> {
    const id = Identifier.create("repository", environment);
    const claim = {
        ...Claim.key(repository, "id", [id]),
        packageId: repository.policy.definition.packageId,
        objectId: id,
        owner: accountId,
    };
    const refused = await Claim.replace(
        universe,
        {
            indexes: [claim.index],
            objectId: id,
            owner: accountId,
            claims: [claim],
            state: "active",
        },
        environment,
    );
    if (refused.length > 0) {
        throw new TypeError(`another object claims repository ${id}`);
    }

    return id;
}

/** Move a repository reference to a commit as the space's replica of the references receives it. */
async function moveReference(
    database: DatabaseConnection,
    moved: {
        readonly id: Identifier<"reference">;
        readonly repository: Identifier<"repository">;
        readonly name: string;
    },
    commit: Commit,
    now: number,
): Promise<void> {
    await database.transaction(async (transaction) => {
        await transaction.log.asReplica(async () => {
            const row = {
                scope: FORGE_HOME,
                parentId: moved.repository,
                name: moved.name,
                object: commit,
                commit,
                observedAt: now,
                updatedAt: now,
            };
            await transaction
                .insert(reference.table)
                .values({ ...row, id: moved.id, createdAt: now })
                .onConflictDoUpdate({ target: reference.table.id, set: row });
        });
    });
}

/** Point a package's tag at a version as the space's replica of the forge's tags receives it. */
async function moveTag(
    database: DatabaseConnection,
    moved: { readonly id: Identifier<"tag">; readonly package: PackageId; readonly name: string },
    version: string,
    now: number,
): Promise<void> {
    await database.transaction(async (transaction) => {
        await transaction.log.asReplica(async () => {
            const row = {
                scope: FORGE_HOME,
                parentId: moved.package,
                name: moved.name,
                version,
                updatedAt: now,
            };
            await transaction
                .insert(tag.table)
                .values({ ...row, id: moved.id, createdAt: now })
                .onConflictDoUpdate({ target: tag.table.id, set: row });
        });
    });
}

/** Read the release version of an installation's targeted build, absent for another build. */
async function targetedVersion(database: DatabaseConnection, id: Identifier<"installation">) {
    const snapshot = Snapshot.live(database);
    const [rolled] = await snapshot.read(rollout.table).findMany({ where: { id } });
    const revision = await snapshot
        .read(installationRevision.table)
        .findFirst({ where: { id: present(rolled?.revisionId, "the targeted revision") } });
    const built = await snapshot
        .read(build.table)
        .findFirst({ where: { id: present(revision, "the revision").buildId } });

    return built?.source.kind === "release" ? built.source.version : undefined;
}

/** Read a space's builds of commits in order, leaving out the releases its stack pulled. */
async function commitBuilds(database: DatabaseConnection) {
    const rows = await database
        .select()
        .from(build.table)
        .orderBy(build.table.createdAt, build.table.id);

    return rows.filter((row) => row.source.kind === "commit");
}

/** List a space's builds of commits in order as their commit and state. */
async function listBuilds(database: DatabaseConnection) {
    return (await commitBuilds(database)).map((row) => [
        row.source.kind === "commit" ? row.source.commit : null,
        row.state,
    ]);
}

/** Serve a space whose installations follow repository references. */
async function serveCommits() {
    // serve installations following references from repositories the universe claims
    const fixture = await TestSpace.open({
        follows: new Forge({ resolve: async () => new Map() }, async () => FORGE_HOME).follows(),
    });
    const { database } = fixture;
    const { clock, environment } = fixture.machine;
    const universe = fixture.universe.storage.database;
    const claims = ClaimLookup.of(Snapshot.live(universe).read(claimTable));
    const forge = new Forge(claims, async () => FORGE_HOME);
    const followed = {
        forge,
        follows: forge.follows(),
        // call the forge in the residency home with a publisher that sends nothing
        publisherAt: () => TestEntitlement.publisher,
    };
    const served = await fixture.serve(followed);
    const owner = served.as(fixture.owner.subject).client(stackPackage);
    const installer = served.as(fixture.owner.subject).client(objectPackage);
    const options = await fixture.options(followed);
    const store = serveSpace(options).store;
    const follow = (id: string) => reconcile(store, "rollout", { id });

    // install a package following the main branch of a repository in the space's account
    const repositoryId = await claimRepository(environment, universe, ids.account);
    const selection = {
        kind: "repository" as const,
        repository: repositoryId,
        reference: "refs/heads/main",
        directory: ".",
    };
    const install = (name: string, packageId: PackageId = ids.package) =>
        installer.installation.create({
            scope: ids.space,
            requestId: RequestId.create(environment),
            packageId,
            selection,
            name,
        });

    // move the branch to a commit as the space's replica of the repository's references receives it
    const id = Identifier.create("reference", fixture.environment);
    const move = (commit: Commit) =>
        moveReference(
            database,
            { id, repository: repositoryId, name: selection.reference },
            commit,
            clock.now(),
        );

    return {
        fixture,
        clock,
        environment,
        database,
        universe,
        owner,
        installer,
        runner: served.as(fixture.machine.id),
        store,
        follow,
        repository: repositoryId,
        selection,
        install,
        move,
        built: () => listBuilds(database),
    };
}

/** Describe the refusal of a selection of a repository the space's account lacks. */
function missing(repositoryId: string): [string, string, string] {
    return [
        "NOT_FOUND",
        "REPOSITORY_NOT_FOUND",
        `the space's account has no repository ${repositoryId}`,
    ];
}

test("build the commit a followed reference resolves to once for the installation selecting it, run by the space's machine, and submit the succeeded build to it", async () => {
    // follow the main branch and move it to a commit
    const { fixture, database, follow, runner, install, move, environment } = await serveCommits();
    const installed = await install("library");
    const commit = Commit.parse("a".repeat(40));
    await move(commit);
    await follow(installed.id);

    // run the commit's build as the space's machine and store it
    const [pending] = await commitBuilds(database);
    const target = { spaceId: ids.space, id: present(pending, "the commit's build").id };
    const client = runner.client(stackPackage);
    await client.build.start({ ...target, requestId: RequestId.create(environment) });
    await client.build.progress({
        ...target,
        requestId: RequestId.create(environment),
        progress: { phase: "building", outputs: ["bun"] },
    });
    await fixture.storeBuild(runner, target, await writeBuild(environment.crypto));

    // submit the succeeded build once reconciled again
    await follow(installed.id);
    const [followed] = await database
        .select({ buildId: installationRevision.table.buildId })
        .from(rollout.table)
        .innerJoin(
            installationRevision.table,
            eq(installationRevision.table.id, rollout.table.revisionId),
        )
        .where(eq(rollout.table.id, installed.id));
    const [only] = await commitBuilds(database);

    expect({
        source: only?.source,
        runner: only?.runnerId !== null,
        followed: followed?.buildId,
    }).toEqual({
        source: {
            kind: "commit",
            repository: only?.source.kind === "commit" ? only.source.repository : null,
            commit,
            directory: ".",
        },
        runner: true,
        followed: only?.id,
    });
});

test("request one replica of the references a served space's installations select, from the forge in its residency home", async () => {
    // follow the main branch from two packages and a tag from a third
    const { installer, store, repository: repositoryId, selection, install } = await serveCommits();
    await install("site");
    await install("preview", PackageId.parse("package-01996ab0-0000-7000-8000-0000000000c1"));
    await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(store.environment),
        packageId: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000c2"),
        selection: { ...selection, reference: "refs/tags/v1" },
        name: "release",
    });

    // request one replica of the forge home with each selected reference once, as the space
    const requests = await present(store.subscriber, "the subscriber").subscriptions(
        Snapshot.live(store.database),
    );
    const keys = ["refs/heads/main", "refs/tags/v1"].map((name) => ({
        repository: repositoryId,
        name,
    }));
    expect(
        requests
            .map((request) => request.subscription)
            .filter((subscription) => subscription.scope === FORGE_HOME),
    ).toEqual([Shape.subscription(FORGE_HOME, [{ shape: referenceShape, keys }], ids.space)]);
});

test("refuse an installation selecting a repository of another account or one no account claims, when created, updated or submitted", async () => {
    // claim a repository of another account and create one the universe has no claim of
    const { universe, owner, installer, selection, install, environment } = await serveCommits();
    const other = schema
        .identifier("account")
        .parse("account-01996ab0-0000-7000-8000-0000000000a2");
    const foreign = {
        ...selection,
        repository: await claimRepository(environment, universe, other),
    };
    const unclaimed = { ...selection, repository: Identifier.create("repository", environment) };

    // refuse creating either and create one selecting the account's own repository
    const create = (chosen: typeof selection) =>
        installer.installation.create({
            scope: ids.space,
            requestId: RequestId.create(environment),
            packageId: ids.package,
            selection: chosen,
            name: "foreign",
        });
    expect([await refusal(create(foreign)), await refusal(create(unclaimed))]).toEqual([
        missing(foreign.repository),
        missing(unclaimed.repository),
    ]);
    const installed = await install("site");

    // refuse moving it to another account's repository by an update or a submission
    const update = installer.installation.update({
        scope: ids.space,
        id: installed.id,
        requestId: RequestId.create(environment),
        revision: installed.revision,
        selection: foreign,
    });
    const submit = owner.rollout.submit({
        spaceId: ids.space,
        id: installed.id,
        requestId: RequestId.create(environment),
        selection: foreign,
        build: { kind: "build", build: Identifier.create("build", environment) },
    });
    expect([await refusal(update), await refusal(submit)]).toEqual([
        missing(foreign.repository),
        missing(foreign.repository),
    ]);
});

test("cancel the unfinished build of a commit a newer commit of the selected branch supersedes, and build a commit followed again anew", async () => {
    // build the first commit of the followed branch
    const { follow, install, move, built } = await serveCommits();
    const installed = await install("site");
    const [first, second] = [Commit.parse("a".repeat(40)), Commit.parse("b".repeat(40))];
    await move(first);
    await follow(installed.id);

    // move the branch on and back to the first commit
    await move(second);
    await follow(installed.id);
    await move(first);
    await follow(installed.id);

    // cancel each superseded build and build the first commit again once it is followed again
    expect(await built()).toEqual([
        [first, "cancelled"],
        [second, "cancelled"],
        [first, "pending"],
    ]);
});

test("create one build of a commit when two reconciliations of its installation create it at once", async () => {
    // follow one branch and reconcile its installation twice at once
    const { follow, install, move, built } = await serveCommits();
    const site = await install("site");
    await move(Commit.parse("a".repeat(40)));
    await Promise.all([follow(site.id), follow(site.id)]);

    // read the one build both create
    expect(await built()).toEqual([["a".repeat(40), "pending"]]);
});

test("follow a selected tag to the release its replicated row points at, and to the next once the tag moves", async () => {
    // point latest at the first release in the space's replica, and install the package following it
    const { database, clock, environment, installer, follow } = await serveCommits();
    const id = Identifier.create("tag", environment);
    const point = (version: string) =>
        moveTag(database, { id, package: ids.package, name: "latest" }, version, clock.now());
    await point("2026.9.0");
    const installed = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(environment),
        packageId: ids.package,
        selection: { kind: "tag", tag: "latest" },
        name: "tagged",
    });
    await follow(installed.id);
    const first = await targetedVersion(database, installed.id);

    // move latest to the next release and follow it there
    await point("2026.10.0");
    await follow(installed.id);

    expect([first, await targetedVersion(database, installed.id)]).toEqual([
        "2026.9.0",
        "2026.10.0",
    ]);
});
