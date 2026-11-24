import { fixtureGraph } from "@destack/build/test";
import { testHost } from "@destack/test/host";
import { Failure } from "@destack/error";
import { principal } from "@destack/access";
import { ObjectReference, Scope, RequestId } from "@destack/sync";
import { BuildReader, BuildImage, PackageManifest } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { Commit, Digest, present } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { BuildSource } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { StackError } from "@destack/stack/error";
import { ids, TestSpace } from "../src/test/index.ts";
import { graph } from "@destack/package";

/** The output the builds compile. */
const OUTPUTS = ["bun"];

/** The commit a clean checkout is at. */
const COMMIT = Commit.parse("c".repeat(40));

/** The package the builds compile. */
const PACKAGE = { id: ids.package, name: "@example/library", version: "2026.9.0" };

/** Write a built package with one module, as a runner compiles it. */
function writeBuild(): Promise<BuildReader> {
    return MemoryBuild.write(
        new Map([["index.js", new TextEncoder().encode("export {};\n")]]),
        { role: "app", package: PACKAGE },
        testHost().crypto,
    );
}

/** List the digests of a build's referenced and stored files. */
async function digests(build: BuildReader): Promise<string[]> {
    return [...build.lists(), ...(await build.files())].map((file) => file.digest);
}

/** Serve a machine with builds in its build store, the owner creating builds it runs. */
async function implementBuild() {
    const fixture = await TestSpace.open();
    await fixture.serve();
    const client = (caller: string) => fixture.as(caller).client(stackPackage);
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
        owner: client(fixture.owner.id),
        runner: client(fixture.machine.id),
        machine: fixture.as(fixture.machine.id),
        create,
    };
}

test("compile a checkout's build on its machine and read its image back from the build store", async () => {
    // create the build, placing its compile on the checkout's machine
    const { fixture, machine, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id };
    const step = await fixture.compileStep(created.id);

    // push every file, the manifest, the image and the graph into the space's empty builds, then complete
    const reader = await writeBuild();
    const root = await Digest.of(PackageManifest.serialize(reader.manifest), testHost().crypto);
    const expected = BuildImage.of(root);
    const imageDigest = await BuildImage.digest(expected, testHost().crypto);
    const graphs = await Promise.all(
        (await fixtureGraph(reader)).map(
            async (module) => (await graph.Module.file(module, testHost().crypto)).digest,
        ),
    );
    const lacking = [
        ...new Set([...(await digests(reader)), root, imageDigest, ...graphs]),
    ].toSorted();
    const uploaded = await fixture.storeBuild(machine, target, reader, { commit: COMMIT });
    const finished = await fixture.finished(created.id);
    const image = present(finished.image, "the stored image");

    // read the image and its root back from the build store
    const space = await fixture.builds();
    const stored = await space.image(image);
    const readBack = await space.contents(stored.root);

    expect({
        created: created.state,
        runner: ObjectReference.at("runner", step),
        uploaded: [...uploaded].toSorted(),
        finished: [finished.state, finished.outputs, finished.commit, image],
        stored,
        read: [readBack.manifest, await readBack.files()],
    }).toEqual({
        created: "running",
        runner: principal.machine.reference(Scope.universe.id, fixture.machine.id),
        uploaded: lacking,
        finished: ["succeeded", OUTPUTS, COMMIT, imageDigest],
        stored: expected,
        read: [reader.manifest, await reader.files()],
    });
});

test("push a succeeded build to the registry as its space's owner, refusing its runner and an unfinished build", async () => {
    // create a build and push it unfinished
    const { fixture, owner, runner, machine, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id };
    const unfinished = await refusal(
        owner.build.push({
            ...target,
            account: ids.account,
            requestId: RequestId.create(fixture.machine.environment),
        }),
    );

    // compile it to its stored image
    await fixture.storeBuild(machine, target, await writeBuild());
    const image = present((await fixture.finished(created.id)).image, "the stored image");

    // push it as the runner and as the owner
    const asRunner = await refusal(
        runner.build.push({
            ...target,
            account: ids.account,
            requestId: RequestId.create(fixture.machine.environment),
        }),
    );
    const pushed = await owner.build.push({
        ...target,
        account: ids.account,
        requestId: RequestId.create(fixture.machine.environment),
    });

    expect({
        unfinished,
        asRunner,
        pushed,
        isInRegistry: await fixture.isPushed(image),
    }).toEqual({
        unfinished: [
            "SERVICE_UNAVAILABLE",
            "BUILD_UNAVAILABLE",
            `build '${created.id}' is unavailable: it is 'running'`,
        ],
        asRunner: [
            "FORBIDDEN",
            "PERMISSION_DENIED",
            "permission 'push' is forbidden: it is not granted",
        ],
        pushed: { image },
        isInRegistry: true,
    });
});

test("refuse ending a build's compile to anyone but its runner, and a compile without its pushed image", async () => {
    // end the compile as the build's owner, then complete it as the runner with an image never pushed
    const { fixture, machine, create } = await implementBuild();
    const created = await create();
    const failure = Failure.http("INTERNAL_SERVER_ERROR", "x");
    const asOwner = await refusal(
        fixture.endCompile(fixture.as(fixture.owner.id), created.id, { method: "fail", failure }),
    );
    const unpushed = await fixture.endCompile(machine, created.id, {
        method: "complete",
        output: { image: "a".repeat(64), outputs: OUTPUTS },
    });

    expect({ asOwner, unpushed: [unpushed.state, unpushed.failure] }).toEqual({
        asOwner: [
            "FORBIDDEN",
            "PERMISSION_DENIED",
            "permission 'end' is forbidden: it is not granted",
        ],
        unpushed: [
            "failed",
            Failure.of(
                StackError.of(
                    "BUILD_NOT_FOUND",
                    `image '${"a".repeat(64)}' not found: it was not pushed to the space`,
                ),
            ),
        ],
    });
});

test("fail a build with the failure its runner reports, and refuse cancelling it once finished", async () => {
    // fail the build's compile as its runner
    const { fixture, owner, machine, create } = await implementBuild();
    const created = await create();
    const target = { spaceId: ids.space, id: created.id };
    const failure = Failure.http("UNPROCESSABLE_CONTENT", "the inspection failed");
    const failed = await fixture.endCompile(machine, created.id, { method: "fail", failure });

    // refuse cancelling the finished build
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
        [
            "PRECONDITION_FAILED",
            "BUILD_BLOCKED",
            `cannot cancel build '${created.id}': it is 'failed'`,
        ],
    ]);
});

test("compile a commit's build on the space's runner, and refuse one on a space no runner serves", async () => {
    // serve the space on a machine running its commit builds, and on one running none
    const fixture = await TestSpace.open();
    const source = BuildSource.parse({
        kind: "commit",
        repository: "repository-01996ab0-0000-7000-8000-000000000071",
        commit: COMMIT,
        directory: ".",
    });
    const create = async (isRunner: boolean, commit: Commit) => {
        await fixture.restart({ isRunner });
        const owner = fixture.as(fixture.owner.subject).client(stackPackage);

        return owner.build.create({
            spaceId: ids.space,
            requestId: RequestId.create(fixture.machine.environment),
            source: BuildSource.parse({ ...source, commit }),
            outputs: OUTPUTS,
        });
    };

    // place the compile on the serving machine, and refuse a commit's build without one
    const placed = await fixture.compileStep((await create(true, COMMIT)).id);
    const unplaced = await refusal(create(false, Commit.parse("d".repeat(40))));
    expect([ObjectReference.at("runner", placed), unplaced]).toEqual([
        principal.machine.reference(Scope.universe.id, fixture.machine.id),
        ["PRECONDITION_FAILED", "RUNNER_BLOCKED", "cannot run the work: this space has no runner"],
    ]);
});
