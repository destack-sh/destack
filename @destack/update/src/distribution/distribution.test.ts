import { Digest } from "@destack/schema";
import { Stage } from "../release/index.ts";
import { testHost } from "@destack/test/host";
import { join } from "node:path";
import { afterAll, beforeAll, expect, test } from "@destack/test";
import { Release } from "../release/index.ts";
import { createRootKey, TestUpdate } from "../test/index.ts";
import { SignedRepository, type SigningKey } from "../publish/index.ts";
import { Manifest } from "../manifest/manifest.ts";
import { Distribution } from "./distribution.ts";

/** Offline signers shared as immutable key material. */
let rootKeys: SigningKey[];
/** The compiled fixture distribution, reporting version 2026.9.1. */
let fixture: string;

beforeAll(async () => {
    rootKeys = await Promise.all(Array.from({ length: 3 }, () => createRootKey(testHost().crypto)));
    fixture = await TestUpdate.compile();
}, 10000);

afterAll(async () => {
    await testHost().files.remove(fixture, { recursive: true });
});

test("stage a newer release once, keeping it staged while none newer is published", async () => {
    // run 2026.9.0 of a distribution following the fixture's repository
    await using repository = await TestUpdate.open(
        fixture,
        rootKeys,
        "2026.9.1",
        TestUpdate.world(),
    );
    const home = await repository.environment.files.makeTemporaryDirectory("destack-distribution-");
    const distribution = new Distribution({
        home,
        repository: repository.options.repository,
        root: SignedRepository.encode(repository.root).toString(),
        running: new Release("2026.9.0", repository.target),
        executable: join(home, "bin", "destack"),
        applications: join(home, "Applications"),
        stage: new Stage("production"),
        environment: repository.environment,
    });
    try {
        // stage the published release, then stage again without a newer one
        const first = await distribution.stage();
        const second = await distribution.stage();

        // expect the release staged both times, and the running one installed
        expect({
            first: first?.release.version,
            second: second?.release.version,
            staged: (await distribution.staged())?.release.version,
            installed: (await distribution.installed()).version,
            application: await distribution.application(),
        }).toEqual({
            first: "2026.9.1",
            second: "2026.9.1",
            staged: "2026.9.1",
            installed: "2026.9.0",
            application: undefined,
        });
    } finally {
        await repository.environment.files.remove(home, { recursive: true });
    }
});

test("check a toolchain out once, from earlier toolchains before the feed, keeping it and the one before", async () => {
    // run a distribution following the fixture's repository, its executable's resources beside it
    await using repository = await TestUpdate.open(
        fixture,
        rootKeys,
        "2026.9.1",
        TestUpdate.world(),
    );
    const { files } = repository.environment;
    const home = await files.makeTemporaryDirectory("destack-distribution-");
    const resources = join(home, "bin");
    const distribution = new Distribution({
        home,
        repository: repository.options.repository,
        root: SignedRepository.encode(repository.root).toString(),
        running: new Release("2026.9.0", repository.target),
        executable: join(resources, "destack"),
        applications: join(home, "Applications"),
        stage: new Stage("production"),
        environment: repository.environment,
    });

    // list three toolchains over the published files: both, the command alone, the desktop alone
    const published = await Manifest.read(repository.source, repository.environment);
    const [command, desktop] = published.files;
    if (command === undefined || desktop === undefined) {
        throw new TypeError("the fixture publishes a command and a desktop");
    }
    const toolchains = [[command, desktop], [command], [desktop]].map((listed) =>
        new Manifest(listed, []).encode(),
    );
    const checkout = async (manifest: string) => {
        const name = Distribution.toolchainManifest(repository.environment.host.architecture);
        await files.writeTextFile(join(resources, name), manifest);
        const { directory } = await distribution.toolchain();
        if (directory === undefined) {
            throw new TypeError("a distribution's toolchain has a directory");
        }

        return directory;
    };
    const listed = async (directory: string) =>
        (await files.readDirectory(directory, { recursive: true }))
            .filter((entry) => entry.kind === "file")
            .map((entry) => entry.name)
            .toSorted();
    try {
        // check the first toolchain out from the feed, then reuse it
        await files.mkdir(resources, { recursive: true });
        const [both, alone, other] = toolchains;
        if (both === undefined || alone === undefined || other === undefined) {
            throw new TypeError("the scenario lists three toolchains");
        }
        const first = await checkout(both);
        const reused = await checkout(both);

        // check the next two out with the feed's blobs gone, copying every file from the first
        await files.remove(join(repository.path, "targets", "blobs"), { recursive: true });
        const second = await checkout(alone);
        const third = await checkout(other);

        // expect each toolchain at its manifest's digest, and only the last two kept
        const root = join(home, "toolchain");
        const { crypto } = repository.environment;
        const directories = await Promise.all(
            [both, alone, other].map(async (text) => join(root, await Digest.of(text, crypto))),
        );
        expect({
            first,
            reused,
            second,
            third,
            kept: (await files.readDirectory(root))
                .map((entry) => join(root, entry.name))
                .toSorted(),
            files: { second: await listed(second), third: await listed(third) },
        }).toEqual({
            first: directories[0],
            reused: directories[0],
            second: directories[1],
            third: directories[2],
            kept: [second, third].toSorted(),
            files: { second: [command.path], third: [desktop.path] },
        });
    } finally {
        await files.remove(home, { recursive: true });
    }
});

test("take a local release's toolchain from its resources", async () => {
    // run a local release whose executable's resources ship its toolchain
    const { files } = testHost();
    const home = await files.makeTemporaryDirectory("destack-distribution-");
    const resources = join(home, "bin");
    const distribution = new Distribution({
        home,
        repository: new URL("http://127.0.0.1/"),
        root: Distribution.root(),
        running: new Release("2026.9.0", "aarch64-apple-darwin"),
        executable: join(resources, "destack"),
        applications: join(home, "Applications"),
        stage: new Stage("local"),
        environment: testHost(),
    });
    try {
        expect((await distribution.toolchain()).directory).toBe(join(resources, "toolchain"));
    } finally {
        await files.remove(home, { recursive: true });
    }
});
