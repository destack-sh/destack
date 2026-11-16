import registryPackage from "@destack/registry/package";
import { fileURLToPath } from "node:url";
import forgePackage from "../src/package.ts";
import { History } from "@destack/git/test";
import { FORGE_HOME_OPTIONS } from "../src/test/index.ts";
import { FOLLOW_TEMPLATE } from "../src/test/index.ts";
import type { Crypto } from "@destack/environment";
import { PackageId } from "@destack/package";
import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { reference, referenceShape } from "@destack/registry/object";
import type { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Commit, present, schema } from "@destack/schema";
import { ObjectReference, RequestId } from "@destack/sync";
import { build, installationRevision, rollout } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import objectPackage from "@destack/object/package";
import { ids, reconcile, TestSpace } from "@destack/machine/test";
import { expect, refusal, test } from "@destack/test";
import { Shape } from "@destack/object";

/** The branch the installations follow. */
const MAIN = "refs/heads/main";

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
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
    const store = await fixture.store();
    const follow = () => fixture.simulation.idle();
    const reconcileTwice = (id: string) =>
        Promise.all(
            [0, 1].map(() => reconcile(store, "destack.rollout.resume", { installationId: id })),
        );

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

    // follow a reference of a repository as the person, the main branch by default
    const follows = async (name = MAIN, from = repositoryId) => {
        const { items } = await forge.reference.list({
            spaceId: home,
            where: { parentId: from, name },
            limit: 1,
        });
        const [row] = items;

        return reference.reference(home, present(row, `reference ${name}`).id);
    };

    // install a package following a reference of the repository
    const install = async (name: string, packageId: PackageId = ids.package, followed = MAIN) =>
        installer.installation.create({
            scope: ids.space,
            requestId: RequestId.create(environment),
            packageId,
            ...ObjectReference.columns("follows", await follows(followed)),
            directory: ".",
            name,
        });

    // write the history to the repository and replicate its references
    const history = new History();
    const write = async () => {
        const written = await history.write(remote);
        await forge.repository.refresh({
            spaceId: home,
            id: repositoryId,
            requestId: RequestId.create(environment),
        });
        await fixture.simulation.idle();

        return written;
    };

    // move the branch to a new commit or a written one
    let commits = 0;
    const moveTo = async (commit?: Commit): Promise<Commit> => {
        if (commit === undefined) {
            commits += 1;
            history.commit("main", "index.ts", `export const commit = ${commits};\n`);
        } else {
            history.reset(MAIN, commit);
        }
        const [written] = await write();

        return Commit.parse(commit ?? present(written, "the written commit"));
    };
    const move = () => moveTo();

    // point another reference at a written commit
    const point = async (name: string, commit: Commit) => {
        history.reset(name, commit);
        await write();
    };

    return {
        fixture,
        residency,
        home,
        clock,
        environment,
        database,
        universe,
        owner,
        installer,
        runner: fixture.as(fixture.machine.id),
        store,
        follow,
        reconcileTwice,
        forge,
        repository: repositoryId,
        follows,
        install,
        move,
        moveTo,
        point,
        built: () => listBuilds(database),
    };
}

/** Describe the refusal of a reference the space cannot read. */
function unreadable(id: string): [string, string, string] {
    return ["NOT_FOUND", "NOT_FOUND", `reference '${id}' not found`];
}

test("submit the build of a followed commit its space's machine ran", async () => {
    // move the main branch to a commit and follow it
    const { fixture, database, follow, runner, install, move, environment } = await serveCommits();
    const commit = await move();
    const installed = await install("library");
    await follow();

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
    await follow();
    const [followed] = await database
        .select({ buildId: installationRevision.table.buildId })
        .from(rollout.table)
        .innerJoin(
            installationRevision.table,
            eq(installationRevision.table.id, rollout.table.targetRevisionId),
        )
        .where(eq(rollout.table.installationId, installed.id));
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

test("request one replica of the references a space's installations follow", async () => {
    // follow the main branch from two packages and a tag from a third
    const { store, follows, install, move, point, home } = await serveCommits();
    const commit = await move();
    await point("refs/tags/v1", commit);
    await install("site");
    await install("preview", PackageId.parse("package-01996ab0-0000-7000-8000-0000000000c1"));
    await install(
        "release",
        PackageId.parse("package-01996ab0-0000-7000-8000-0000000000c2"),
        "refs/tags/v1",
    );

    // request one replica of each followed reference
    const requests = await present(store.subscriber, "the subscriber").subscriptions(
        Snapshot.live(store.database),
    );
    const keys = [await follows(), await follows("refs/tags/v1")].map(({ id }) => ({ id }));
    expect(
        requests
            .map((request) => request.subscription)
            .filter((subscription) => subscription.scope === home),
    ).toEqual([Shape.subscription(home, [{ shape: referenceShape, keys }], ids.space)]);
});

test("refuse an installation following another account's reference", async () => {
    // push the main branch of another account's repository
    const { fixture, home, forge, installer, follows, install, move, environment } =
        await serveCommits();
    await move();
    const other = schema
        .identifier("account")
        .parse("account-01996ab0-0000-7000-8000-0000000000a2");
    await TestSpace.account(fixture.universe, fixture.owner, { id: other, handle: "other" });
    const created = await forge.repository.create({
        spaceId: home,
        accountId: other,
        requestId: RequestId.create(environment),
        name: "foreign",
        hosting: "platform",
    });
    const history = new History();
    history.commit("main", "index.ts", "export const commit = 0;\n");
    await history.write(fileURLToPath(present(created.remote, "the foreign repository's remote")));
    await forge.repository.refresh({
        spaceId: home,
        id: created.id,
        requestId: RequestId.create(environment),
    });
    const foreign = await follows(MAIN, created.id);

    // refuse it, and create one following the account's own repository
    const create = (followed: ObjectReference) =>
        installer.installation.create({
            scope: ids.space,
            requestId: RequestId.create(environment),
            packageId: ids.package,
            ...ObjectReference.columns("follows", followed),
            directory: ".",
            name: "foreign",
        });
    expect(await refusal(create(foreign))).toEqual(unreadable(foreign.id));
    const installed = await install("site");

    // refuse moving it to another account's repository
    const update = installer.installation.update({
        scope: ids.space,
        id: installed.id,
        requestId: RequestId.create(environment),
        revision: installed.revision,
        ...ObjectReference.columns("follows", foreign),
    });
    expect(await refusal(update)).toEqual(unreadable(foreign.id));
});

test("build each commit a followed branch moves to once", async () => {
    // follow the branch's first commit, reconciling its installation twice at once
    const { reconcileTwice, follow, install, move, moveTo, built } = await serveCommits();
    const first = await move();
    const site = await install("site");
    await follow();
    await reconcileTwice(site.id);

    // read the one build both reconciliations create
    expect(await built()).toEqual([[first, "pending"]]);

    // move the branch on and back to the first commit
    const second = await move();
    await follow();
    await moveTo(first);
    await follow();

    // cancel each superseded build and build the first commit again once it is followed again
    expect(await built()).toEqual([
        [first, "cancelled"],
        [second, "cancelled"],
        [first, "pending"],
    ]);
});
