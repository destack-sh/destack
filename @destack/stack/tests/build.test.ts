import { principal } from "@destack/access";
import { machine } from "@destack/account/object";
import { ObjectReference, Scope, RequestId } from "@destack/sync";
import { type DatabaseConnection } from "@destack/db";
import { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Digest, present } from "@destack/schema";
import { createClient } from "@destack/object/client";
import { expect, refusal, test } from "@destack/test";
import { BuildSource } from "../src/object/index.ts";
import stackPackage from "../src/package.ts";
import { ids, StackFixture } from "../src/test/index.ts";

/** The output the builds compile. */
const OUTPUTS = ["bun"];

/** The commit a clean checkout is at. */
const COMMIT = "c".repeat(40);

/** The package the builds compile. */
const PACKAGE = { id: ids.package, name: "@example/library", version: "2026.9.0" };

/** Write a built package with one module, as a runner compiles it. */
function writeBuild(): Promise<BuildReader> {
    return MemoryBuild.write(new Map([["index.js", new TextEncoder().encode("export {};\n")]]), {
        package: PACKAGE,
    });
}

/** Replicate the machine's own row into its database, as the universe's replicas carry it. */
async function replicateMachine(database: DatabaseConnection): Promise<void> {
    const now = Date.now();
    await database.transaction(async (transaction) => {
        await transaction.log.asReplica(async () => {
            await transaction.insert(machine.table).values({
                id: ids.machine,
                scope: ids.account,
                name: "laptop",
                kind: "personal",
                platform: "macOS",
                architecture: "arm64",
                model: "Mac14,2",
                createdAt: now,
                updatedAt: now,
            });
        });
    });
}

/** Serve a machine keeping builds in its build store, the owner creating builds it runs. */
async function implementBuild() {
    const fixture = await StackFixture.open();
    await replicateMachine(fixture.database);
    const { server } = await fixture.server({ machine: ids.machine });
    const served = (caller: string) =>
        createClient(stackPackage, StackFixture.endpoint(server, caller));
    const create = () =>
        served(ids.owner).build.create({
            spaceId: ids.space,
            requestId: RequestId.create(Date.now()),
            source: {
                kind: "checkout",
                machine: ids.machine,
                checkout: ids.checkout,
                directory: ".",
            },
            outputs: OUTPUTS,
        });

    return { fixture, server, owner: served(ids.owner), runner: served(ids.machine), create };
}

test("run a checkout's build as its machine and read it back from the build store", async () => {
    // create the build, naming the checkout's machine its runner
    const { fixture, runner, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id };

    // advance it as the runner through compiling
    await runner.build.start({ ...target, requestId: RequestId.create(Date.now()) });
    await runner.build.progress({
        ...target,
        requestId: RequestId.create(Date.now()),
        progress: { phase: "building", outputs: OUTPUTS, commit: COMMIT },
    });

    // lease every file the space lacks, store them and finish with the manifest
    const reader = await writeBuild();
    const uploads = await fixture.storeBuild(runner, target, reader);
    const finished = await runner.build.get(target);
    const manifest = present(finished.manifest, "the stored manifest");

    // read the build back from the build store
    const readBack = await (await fixture.builds()).packages.contents(manifest);

    expect({
        runner: ObjectReference.at("runner", created),
        leased: uploads.map((upload) => upload.digest).toSorted(),
        finished: [finished.state, finished.commit, manifest],
        read: [readBack.manifest, await readBack.files()],
    }).toEqual({
        runner: principal.machine.reference(Scope.universe.id, ids.machine),
        leased: [
            ...new Set(
                [...reader.references(), ...(await reader.files())].map((file) => file.digest),
            ),
        ].toSorted(),
        finished: ["succeeded", COMMIT, await Digest.of(JSON.stringify(reader.manifest))],
        read: [reader.manifest, await reader.files()],
    });
});

test("push a succeeded build to the registry as its space's owner, refusing its runner and an unfinished build", async () => {
    // create a build and push it unfinished
    const { fixture, owner, runner, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id };
    const unfinished = await refusal(
        owner.build.push({ ...target, requestId: RequestId.create(Date.now()) }),
    );

    // run it to its stored manifest as the runner
    await runner.build.start({ ...target, requestId: RequestId.create(Date.now()) });
    await runner.build.progress({
        ...target,
        requestId: RequestId.create(Date.now()),
        progress: { phase: "building", outputs: OUTPUTS, commit: COMMIT },
    });
    await fixture.storeBuild(runner, target, await writeBuild());
    const manifest = present((await runner.build.get(target)).manifest, "the stored manifest");

    // push it as the runner and as the owner
    const asRunner = await refusal(
        runner.build.push({ ...target, requestId: RequestId.create(Date.now()) }),
    );
    const pushed = await owner.build.push({ ...target, requestId: RequestId.create(Date.now()) });

    expect({
        unfinished,
        asRunner,
        pushed,
        isInRegistry: await fixture.registry.contains(manifest),
    }).toEqual({
        unfinished: ["CONFLICT", `build ${created.id} is pending`],
        asRunner: ["FORBIDDEN", "permission denied: push"],
        pushed: { manifest },
        isInRegistry: true,
    });
});

test("refuse advancing a build to anyone but its runner, and a build not running", async () => {
    // create a build and try advancing it as its creator and before it starts
    const { owner, runner, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id, requestId: RequestId.create(Date.now()) };
    const asOwner = await refusal(owner.build.start(target));
    const unstarted = await refusal(
        runner.build.finish({
            ...target,
            end: { kind: "failed", failure: { code: "INTERNAL_SERVER_ERROR", message: "x" } },
        }),
    );

    expect({ asOwner, unstarted }).toEqual({
        asOwner: ["FORBIDDEN", "permission denied: run"],
        unstarted: ["CONFLICT", `build ${created.id} is pending`],
    });
});

test("fail a running build with the failure its runner reports, and refuse cancelling it once finished", async () => {
    // start and fail the build as its runner
    const { owner, runner, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id };
    await runner.build.start({ ...target, requestId: RequestId.create(Date.now()) });
    const failure = { code: "UNPROCESSABLE_CONTENT" as const, message: "the inspection failed" };
    await runner.build.finish({
        ...target,
        requestId: RequestId.create(Date.now()),
        end: { kind: "failed", failure },
    });

    // read the failure and refuse cancelling the finished build
    const failed = await owner.build.get(target);
    expect([
        [failed.state, failed.failure],
        await refusal(owner.build.cancel({ ...target, requestId: RequestId.create(Date.now()) })),
    ]).toEqual([
        ["failed", failure],
        ["CONFLICT", `build ${created.id} is failed`],
    ]);
});

test("run a commit's build on the space's build runner, and refuse one on a space no build runner serves", async () => {
    // serve the space on a machine running its commit builds, and on one running none
    const fixture = await StackFixture.open();
    await replicateMachine(fixture.database);
    const source = BuildSource.parse({
        kind: "commit",
        repository: "repository-01996ab0-0000-7000-8000-000000000071",
        commit: COMMIT,
        directory: ".",
    });
    const create = async (buildRunner: typeof ids.machine | null) => {
        const { server } = await fixture.server({ machine: ids.machine, buildRunner });
        const owner = createClient(stackPackage, StackFixture.endpoint(server, ids.owner));

        return owner.build.create({
            spaceId: ids.space,
            requestId: RequestId.create(Date.now()),
            source,
            outputs: OUTPUTS,
        });
    };

    // name the build runner, and refuse a commit's build without one
    const created = await create(ids.machine);
    const runner = principal.machine.reference(Scope.universe.id, ids.machine);
    expect([ObjectReference.at("runner", created), await refusal(create(null))]).toEqual([
        runner,
        ["PRECONDITION_FAILED", "no build runner serves this space"],
    ]);
});
