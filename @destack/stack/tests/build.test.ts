import { testHost } from "@destack/test/host";
import { Failure } from "@destack/error";
import { principal } from "@destack/access";
import { ObjectReference, Scope, RequestId } from "@destack/sync";
import { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Digest, present } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { BuildSource } from "../src/object/index.ts";
import stackPackage from "../src/package.ts";
import { ids, TestSpace } from "../src/test/index.ts";

/** The output the builds compile. */
const OUTPUTS = ["bun"];

/** The commit a clean checkout is at. */
const COMMIT = "c".repeat(40);

/** The package the builds compile. */
const PACKAGE = { id: ids.package, name: "@example/library", version: "2026.9.0" };

/** Write a built package with one module, as a runner compiles it. */
function writeBuild(): Promise<BuildReader> {
    return MemoryBuild.write(
        new Map([["index.js", new TextEncoder().encode("export {};\n")]]),
        {
            package: PACKAGE,
        },
        testHost().crypto,
    );
}

/** List the digests of a build's referenced and stored files. */
async function digests(build: BuildReader): Promise<string[]> {
    return [...build.references(), ...(await build.files())].map((file) => file.digest);
}

/** Serve a machine with builds in its build store, the owner creating builds it runs. */
async function implementBuild() {
    const fixture = await TestSpace.open();
    const served = await fixture.serve({ machine: fixture.machine.id });
    const client = (caller: string) => served.as(caller).client(stackPackage);
    const create = () =>
        client(fixture.owner.id).build.create({
            spaceId: ids.space,
            requestId: RequestId.create(fixture.machine.environment),
            source: {
                kind: "checkout",
                machine: fixture.machine.id,
                checkout: ids.checkout,
                directory: ".",
            },
            outputs: OUTPUTS,
        });

    return {
        fixture,
        served,
        owner: client(fixture.owner.id),
        runner: client(fixture.machine.id),
        create,
    };
}

test("run a checkout's build as its machine and read it back from the build store", async () => {
    // create the build, naming the checkout's machine its runner
    const { fixture, runner, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id };

    // advance it as the runner through compiling
    await runner.build.start({
        ...target,
        requestId: RequestId.create(fixture.machine.environment),
    });
    await runner.build.progress({
        ...target,
        requestId: RequestId.create(fixture.machine.environment),
        progress: { phase: "building", outputs: OUTPUTS, commit: COMMIT },
    });

    // lease every file the build store lacks in addition to the space's stack build, store them and finish
    const reader = await writeBuild();
    const stored = new Set(await digests(fixture.releases.space));
    const lacking = [...new Set(await digests(reader))]
        .filter((digest) => !stored.has(digest))
        .toSorted();
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
        runner: principal.machine.reference(Scope.universe.id, fixture.machine.id),
        leased: lacking,
        finished: [
            "succeeded",
            COMMIT,
            await Digest.of(JSON.stringify(reader.manifest), testHost().crypto),
        ],
        read: [reader.manifest, await reader.files()],
    });
});

test("push a succeeded build to the registry as its space's owner, refusing its runner and an unfinished build", async () => {
    // create a build and push it unfinished
    const { fixture, owner, runner, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id };
    const unfinished = await refusal(
        owner.build.push({ ...target, requestId: RequestId.create(fixture.machine.environment) }),
    );

    // run it to its stored manifest as the runner
    await runner.build.start({
        ...target,
        requestId: RequestId.create(fixture.machine.environment),
    });
    await runner.build.progress({
        ...target,
        requestId: RequestId.create(fixture.machine.environment),
        progress: { phase: "building", outputs: OUTPUTS, commit: COMMIT },
    });
    await fixture.storeBuild(runner, target, await writeBuild());
    const manifest = present((await runner.build.get(target)).manifest, "the stored manifest");

    // push it as the runner and as the owner
    const asRunner = await refusal(
        runner.build.push({ ...target, requestId: RequestId.create(fixture.machine.environment) }),
    );
    const pushed = await owner.build.push({
        ...target,
        requestId: RequestId.create(fixture.machine.environment),
    });

    expect({
        unfinished,
        asRunner,
        pushed,
        isInRegistry: await fixture.registry.contains(manifest),
    }).toEqual({
        unfinished: ["CONFLICT", "BUILD_NOT_READY", `build ${created.id} is pending`],
        asRunner: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: push"],
        pushed: { manifest },
        isInRegistry: true,
    });
});

test("refuse advancing a build to anyone but its runner, and a build not running", async () => {
    // create a build and try advancing it as its creator and before it starts
    const { fixture, owner, runner, create } = await implementBuild();
    const created = await create();
    const target = {
        spaceId: ids.space,
        id: created.id,
        requestId: RequestId.create(fixture.machine.environment),
    };
    const asOwner = await refusal(owner.build.start(target));
    const unstarted = await refusal(
        runner.build.finish({
            ...target,
            end: { kind: "failed", failure: Failure.http("INTERNAL_SERVER_ERROR", "x") },
        }),
    );

    expect({ asOwner, unstarted }).toEqual({
        asOwner: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: run"],
        unstarted: ["CONFLICT", "BUILD_STATE_CONFLICT", `build ${created.id} is pending`],
    });
});

test("fail a running build with the failure its runner reports, and refuse cancelling it once finished", async () => {
    // start and fail the build as its runner
    const { fixture, owner, runner, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id };
    await runner.build.start({
        ...target,
        requestId: RequestId.create(fixture.machine.environment),
    });
    const failure = Failure.http("UNPROCESSABLE_CONTENT", "the inspection failed");
    await runner.build.finish({
        ...target,
        requestId: RequestId.create(fixture.machine.environment),
        end: { kind: "failed", failure },
    });

    // read the failure and refuse cancelling the finished build
    const failed = await owner.build.get(target);
    expect([
        [failed.state, failed.failure],
        await refusal(
            owner.build.cancel({
                ...target,
                requestId: RequestId.create(fixture.machine.environment),
            }),
        ),
    ]).toEqual([
        ["failed", failure],
        ["CONFLICT", "BUILD_STATE_CONFLICT", `build ${created.id} is failed`],
    ]);
});

test("run a commit's build on the space's build runner, and refuse one on a space no build runner serves", async () => {
    // serve the space on a machine running its commit builds, and on one running none
    const fixture = await TestSpace.open();
    const source = BuildSource.parse({
        kind: "commit",
        repository: "repository-01996ab0-0000-7000-8000-000000000071",
        commit: COMMIT,
        directory: ".",
    });
    const create = async (buildRunner: typeof fixture.machine.id | null) => {
        const served = await fixture.serve({ machine: fixture.machine.id, buildRunner });
        const owner = served.as(fixture.owner.subject).client(stackPackage);

        return owner.build.create({
            spaceId: ids.space,
            requestId: RequestId.create(fixture.machine.environment),
            source,
            outputs: OUTPUTS,
        });
    };

    // name the build runner, and refuse a commit's build without one
    const created = await create(fixture.machine.id);
    const runner = principal.machine.reference(Scope.universe.id, fixture.machine.id);
    expect([ObjectReference.at("runner", created), await refusal(create(null))]).toEqual([
        runner,
        ["PRECONDITION_FAILED", "BUILD_RUNNER_MISSING", "this space has no build runner"],
    ]);
});
