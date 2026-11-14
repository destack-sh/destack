import registryPackage from "@destack/registry/package";
import { fileURLToPath } from "node:url";
import forgePackage from "../src/package.ts";
import { History } from "@destack/git/test";
import { FORGE_HOME_OPTIONS } from "../src/test/index.ts";
import { FOLLOW_TEMPLATE } from "../src/test/index.ts";
import type { Crypto } from "@destack/environment";
import type { Environment } from "@destack/environment";
import { PackageId } from "@destack/package";
import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { packageObject, referenceShape, repository, tag } from "@destack/registry/object";
import type { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Commit, Identifier, present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { build, installationRevision } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import objectPackage from "@destack/object/package";
import { ids, reconcile, TestSpace } from "@destack/machine/test";
import { expect, refusal, test } from "@destack/test";
import { Claim, claimTable, installation, Shape } from "@destack/object";

/** Write a built package with one module, as a runner compiles it. */
function writeBuild(crypto: Crypto): Promise<BuildReader> {
    return MemoryBuild.write(
        new Map([["index.js", new TextEncoder().encode("export {};\n")]]),
        {
            role: "app",
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
    const refused = await universe.transaction((transaction) =>
        Claim.replace(
            transaction,
            {
                indexes: [claim.index],
                objectId: id,
                owner: accountId,
                claims: [claim],
                state: "active",
            },
            environment,
        ),
    );
    if (refused.length > 0) {
        throw new TypeError(`another object claims repository '${id}'`);
    }

    return id;
}

/** Point a package's tag at a version as the space's replica of the forge's tags receives it. */
async function moveTag(
    database: DatabaseConnection,
    home: Identifier<"space">,
    moved: { readonly id: Identifier<"tag">; readonly package: PackageId; readonly name: string },
    version: string,
    now: number,
): Promise<void> {
    await database.transaction(async (transaction) => {
        await transaction.log.asReplica(async () => {
            const row = {
                scope: home,
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
    const installed = await snapshot.read(installation.table).findFirst({ where: { id } });
    const revision = await snapshot
        .read(installationRevision.table)
        .findFirst({ where: { id: present(installed?.revisionId, "the targeted revision") } });
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

/** List a store's follows of tags and their packages' claims: scope, subject and query. */
async function tagFollows(
    store: Awaited<ReturnType<typeof serveCommits>>["store"],
    database: DatabaseConnection,
) {
    const subscriber = present(store.subscriber, "the subscriber");
    const entries = await subscriber.subscriptions(Snapshot.live(database));

    return entries
        .flatMap(({ subscription }) =>
            Object.keys(subscription.parameters["queries"] ?? {}).map((query) => ({
                scope: subscription.scope,
                subject: subscription.subject?.id ?? null,
                query,
            })),
        )
        .filter(({ query }) => query.startsWith("claim:") || query.startsWith("tag:"))
        .toSorted((left, right) => left.query.localeCompare(right.query));
}

/** Serve a space whose installations follow repository references. */
async function serveCommits() {
    // serve the forge in the residency's home, and installations following references in the fixture's space
    const { spaces } = await TestSpace.copy(FOLLOW_TEMPLATE, FORGE_HOME_OPTIONS);
    const fixture = present(spaces.get(ids.space), "the fixture's space");
    const residency = present(
        [...spaces.values()].find((each) => each.id !== ids.space),
        "the residency's home",
    );
    const home = residency.id;
    const { database } = fixture;
    const { clock, environment } = fixture.machine;
    const universe = fixture.universe.storage.database;
    const served = await fixture.serve();
    const owner = served.as(fixture.owner.subject).client(stackPackage);
    const installer = served.as(fixture.owner.subject).client(objectPackage);
    const { store } = await fixture.spaceStore();
    const follow = (id: string) => reconcile(store, "rollout", { installationId: id });

    // create a platform repository of the space's account at the forge, as the account's owner
    const forge = (await residency.serve())
        .as(fixture.owner.subject)
        .client(registryPackage, forgePackage);
    const created = await forge.repository.create({
        spaceId: home,
        accountId: ids.account,
        requestId: RequestId.create(environment),
        name: "library",
        hosting: "platform",
    });
    const repositoryId = created.id;
    const remote = fileURLToPath(present(created.remote, "the platform repository's remote"));

    // install a package following the main branch of the repository
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

    // move the branch at the forge to a new commit or a written one, then let the space replicate it
    const history = new History();
    let commits = 0;
    const moveTo = async (commit?: Commit): Promise<Commit> => {
        if (commit === undefined) {
            commits += 1;
            history.commit("main", "index.ts", `export const commit = ${commits};\n`);
        } else {
            history.reset("refs/heads/main", commit);
        }
        const [written] = await history.write(remote);
        await forge.repository.refresh({
            spaceId: home,
            id: repositoryId,
            requestId: RequestId.create(environment),
        });
        await fixture.simulation.idle();

        return Commit.parse(commit ?? present(written, "the written commit"));
    };
    const move = () => moveTo();

    return {
        fixture,
        home,
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
        moveTo,
        built: () => listBuilds(database),
    };
}

/** Describe the refusal of a selection of a repository the space's account lacks. */
function missing(repositoryId: string): [string, string, string] {
    return [
        "NOT_FOUND",
        "REPOSITORY_NOT_FOUND",
        `repository '${repositoryId}' of the space's account not found`,
    ];
}

test("build the commit a followed reference resolves to once for the installation selecting it, run by the space's machine, and submit the succeeded build to it", async () => {
    // follow the main branch and move it to a commit
    const { fixture, database, follow, runner, install, move, environment } = await serveCommits();
    const installed = await install("library");
    const commit = await move();
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
        .from(installation.table)
        .innerJoin(
            installationRevision.table,
            eq(installationRevision.table.id, installation.table.revisionId),
        )
        .where(eq(installation.table.id, installed.id));
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
    const {
        installer,
        store,
        repository: repositoryId,
        selection,
        install,
        home,
    } = await serveCommits();
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
            .filter((subscription) => subscription.scope === home),
    ).toEqual([Shape.subscription(home, [{ shape: referenceShape, keys }], ids.space)]);
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
    expect(await Promise.all([refusal(update), refusal(submit)])).toEqual([
        missing(foreign.repository),
        missing(foreign.repository),
    ]);
});

test("cancel the unfinished build of a commit a newer commit of the selected branch supersedes, and build a commit followed again anew", async () => {
    // build the first commit of the followed branch
    const { follow, install, move, moveTo, built } = await serveCommits();
    const installed = await install("site");
    const first = await move();
    await follow(installed.id);

    // move the branch on and back to the first commit
    const second = await move();
    await follow(installed.id);
    await moveTo(first);
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
    const commit = await move();
    await Promise.all([follow(site.id), follow(site.id)]);

    // read the one build both create
    expect(await built()).toEqual([[commit, "pending"]]);
});

test("follow a selected tag to the release its replicated row points at, and to the next once the tag moves", async () => {
    // point latest at the first release in the space's replica, and install the package following it
    const { database, clock, environment, installer, follow, home } = await serveCommits();
    const id = Identifier.create("tag", environment);
    const point = (version: string) =>
        moveTag(database, home, { id, package: ids.package, name: "latest" }, version, clock.now());
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

test("follow a tag's package claim as the machine, then the tag at its forge home", async () => {
    // follow latest of a package whose claim the space lacks
    const { database, environment, fixture, installer, store, home } = await serveCommits();
    const packageId = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000c3");
    await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(environment),
        packageId,
        selection: { kind: "tag", tag: "latest" },
        name: "tagged",
    });

    // read the follows before the claim arrives
    const index = Claim.index(packageObject, "id");
    const claimed = Claim.key(packageObject, "id", [packageId]);
    const before = await tagFollows(store, database);

    // replicate the package's claim into the space, as the machine's follow receives it
    await database.transaction(async (transaction) => {
        await transaction.insert(claimTable).values({
            id: Identifier.create("claim", environment),
            scope: Scope.universe.id,
            index,
            key: claimed.key,
            packageId: packageObject.policy.definition.packageId,
            objectId: packageId,
            owner: ids.account,
            state: "active",
        });
    });
    const after = await tagFollows(store, database);

    // follow the claim at the universe as the machine, then the tag at the forge home as the space
    const claim = {
        scope: Scope.universe.id,
        subject: fixture.machine.id,
        query: `claim:${index}:${claimed.key}:0:claim`,
    };
    const tagged = { scope: home, subject: null, query: `tag:latest:${packageId}:0:tag` };
    expect({ before, after }).toEqual({ before: [claim], after: [claim, tagged] });
});
