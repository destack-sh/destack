import objectPackage from "@destack/object/package";
import type { TestPerson } from "@destack/account/test";
import { expect, refusal, test } from "@destack/test";
import { BuildObject, BuildStore, PackageClient } from "@destack/build/store";
import { TestBuildStore } from "@destack/build/test";
import { Lock, PackageManifest } from "@destack/package/manifest";
import { Digest } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { UNRELEASED_MILLISECONDS } from "../src/server/index.ts";
import { account } from "@destack/account/object";
import forgePackage from "../src/package.ts";
import { ACCOUNT_ID, TestForge } from "../src/test/index.ts";
import { fixtureBuild } from "./fixture/build.ts";
import { MemoryBuild } from "@destack/package/test";
import { PackageName, PackageVersion } from "@destack/package";
import { Tarball, TarballEntry } from "@destack/package/archive";
import { testHost } from "@destack/test/host";

/** The email a person without access to the account signs in with. */
const STRANGER_EMAIL = "stranger@example.test";

/** How long an unreferenced blob stays before deletion: a blob store's hour of grace. */
const GRACE_MILLISECONDS = 60 * 60_000;

/** The npm release whose unit the closure package depends on. */
const TINY = { name: "tiny", version: "1.0.0", integrity: "sha512-dGlueQ==" };

/** The package whose build depends on the tiny npm unit. */
const CLOSURE = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-00000000c105",
    name: "@example/closure",
    version: "2026.9.0",
});

test("serve a release's build by its digest to its package's readers only, yanked or not", async () => {
    // publish a private package, and copy its build by digest into another store as the owner
    const forge = await TestForge.open();
    const published = await forge.publish((await fixtureBuild("answer")).reader);
    const { environment } = forge.home;
    await using copied = await TestBuildStore.create(environment);
    const root = { kind: "manifest", digest: published.manifest } as const;
    const source = new PackageClient(
        `${forge.origin}${forgePackage.mounts.builds.path}/${BuildObject.path(root)}/`,
        forge.endpoint(forge.owner).fetch,
    );
    await copied.pull(source, root, "all");

    // refuse anonymous and other callers and an unknown build, and serve the build once yanked
    const stranger = await forge.signIn(STRANGER_EMAIL);
    const status = async (digest: Digest, person?: TestPerson) => {
        const response = await as(forge, person)(
            new URL(`${forge.origin}${forgePackage.mounts.builds.path}/${digest}/manifest.json`),
            {},
        );
        await response.body?.cancel();

        return response.status;
    };
    const refused = [
        await status(published.manifest),
        await status(published.manifest, stranger),
        await status(Digest.parse("0".repeat(64)), forge.owner),
    ];
    await forge.client(forge.owner).release.yank({
        spaceId: forge.space,
        id: published.release.id,
        requestId: RequestId.create(forge.environment),
        isYanked: true,
    });
    expect({
        copied: (await copied.contents(published.manifest)).manifest,
        refused,
        yanked: await status(published.manifest, forge.owner),
    }).toEqual({
        copied: published.reader.manifest,
        refused: [404, 404, 404],
        yanked: 200,
    });
});

test("refuse uploads outside an account, and a manifest push from a caller who may not publish in the account", async () => {
    // publish the package as its owner and push its next build's files as a stranger
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    const manifest = PackageManifest.serialize(build.manifest);
    const digest = await Digest.of(manifest, testHost().crypto);
    const stranger = await forge.signIn(STRANGER_EMAIL);

    // refuse the owner's manifest outside an account
    const builds = `${forge.origin}${forgePackage.mounts.builds.path}`;
    const post = (person: TestPerson, path: string) =>
        as(forge, person)(new URL(`${builds}/${path}manifests`), {
            method: "POST",
            body: Tarball.stream(
                (async function* () {
                    yield TarballEntry.of(digest, manifest);
                })(),
            ),
            headers: { "content-type": "application/x-tar" },
        });
    const outside = await post(forge.owner, "");
    const pushed = await post(
        stranger,
        `${PackageName.split(build.manifest.package.name).handle}/`,
    );
    expect({
        outside: [outside.status, await outside.json()],
        pushed: [pushed.status, await pushed.json()],
        isStored: await (await forge.store()).contains(digest),
    }).toEqual({
        outside: [
            404,
            {
                defined: true,
                code: "NOT_FOUND",
                status: 404,
                message: "upload target account not found",
                reason: "UPLOAD_OUTSIDE_ACCOUNT_NOT_FOUND",
                domain: "@destack/forge",
            },
        ],
        pushed: [
            404,
            {
                defined: true,
                code: "NOT_FOUND",
                status: 404,
                message: "upload target account not found",
                reason: "UPLOAD_OUTSIDE_ACCOUNT_NOT_FOUND",
                domain: "@destack/forge",
            },
        ],
        isStored: false,
    });
});

test("stage a machine's push without releasing it", async () => {
    // publish the package, and build its next version
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    const digest = await Digest.of(
        PackageManifest.serialize(build.manifest),
        forge.environment.crypto,
    );

    // push it as the forge's machine
    const { url } = forge.endpoint();
    const { fetch } = forge.served.as(forge.home.machine.id).endpoint(forgePackage);
    const builds = (packageId: string) =>
        new PackageClient(`${url}${forgePackage.mounts.builds.path}/${packageId}/`, fetch);
    const handle = PackageName.split(build.manifest.package.name).handle;
    await using pushed = await TestBuildStore.create(forge.environment);
    const lock = await pushed.putLock(Lock.of(await pushed.put(build.reader)));
    const staged = await refusal(builds(handle).push(pushed, [lock]));

    // store the staged build, and release nothing
    const released = await forge.client(forge.owner).release.list({ spaceId: forge.space });
    expect({
        staged,
        isStored: await (await forge.store()).contains(digest),
        versions: released.items.map((release) => release.version),
    }).toEqual({
        staged: "done",
        isStored: true,
        versions: ["2026.9.0"],
    });
});

test("authorize a manifest push from the forge's replica of the account, following a publisher relation the account service grants", async () => {
    // publish the package as its owner, and store its next build to push as a stranger
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    const digest = await Digest.of(
        PackageManifest.serialize(build.manifest),
        forge.environment.crypto,
    );
    const stranger = await forge.signIn(STRANGER_EMAIL);
    await using pushed = await TestBuildStore.create(forge.environment);
    const lock = await pushed.putLock(Lock.of(await pushed.put(build.reader)));
    const push = async () =>
        refusal(
            forge
                .builds(PackageName.split(build.reader.manifest.package.name).handle, stranger)
                .push(pushed, [lock]),
        );

    // refuse the stranger, make them a publisher at the account service and push once the replica follows
    const refused = await push();
    await forge.owner.client(objectPackage).relationship.create({
        scope: forge.owner.subject.id,
        requestId: RequestId.create(forge.environment),
        object: account.reference(forge.owner.subject.id, ACCOUNT_ID),
        relation: "publisher",
        subject: stranger.subject,
    });
    await forge.simulation.idle();

    // refuse the stranger's push at its first request, asking for the missing files
    const uploads = `${forgePackage.mounts.builds.path}/${PackageName.split(build.manifest.package.name).handle}`;
    const answer = JSON.stringify({
        defined: true,
        code: "NOT_FOUND",
        status: 404,
        message: "upload target account not found",
        reason: "UPLOAD_OUTSIDE_ACCOUNT_NOT_FOUND",
        domain: "@destack/forge",
    });
    expect(refused).toEqual([
        "BAD_GATEWAY",
        "UPLOAD_UNREACHABLE",
        `could not call '${uploads}/has': HTTP 404 '${answer}'`,
    ]);
    expect({
        accepted: await push(),
        isStored: await (await forge.store()).contains(digest),
    }).toEqual({
        accepted: "done",
        isStored: true,
    });
});

test("collect a pushed but unreleased build a grace after its lease ends, leaving released builds", async () => {
    // publish one build and push the next without releasing it
    const forge = await TestForge.open();
    const published = await forge.publish((await fixtureBuild("answer")).reader, "public");
    const next = (await fixtureBuild("answer", "2026.9.1")).reader;
    await forge.push(next);
    const unreleased = await BuildStore.digest(next.manifest, forge.environment.crypto);
    const store = await forge.store();
    const stored = async () => ({
        released: await store.contains(published.manifest),
        unreleased: await store.contains(unreleased),
    });

    // retain the unreleased build within its lease, and free it once the lease ends
    const { clock } = forge.environment;
    await forge.simulation.jump(clock.now());
    const within = await stored();
    await forge.simulation.jump(clock.now() + UNRELEASED_MILLISECONDS + 1000);

    // delete the manifest a grace after the collection found it free, then its files a grace later
    await forge.simulation.jump(clock.now() + GRACE_MILLISECONDS);
    await forge.simulation.jump(clock.now() + GRACE_MILLISECONDS);
    expect({ within, after: await stored() }).toEqual({
        within: {
            released: true,
            unreleased: true,
        },
        after: {
            released: true,
            unreleased: false,
        },
    });

    // read and verify every file of the released build after the collection
    const files = await (await store.contents(published.manifest)).distributed();
    const sizes = await Promise.all(files.map(async (file) => (await store.file(file)).length));
    expect(sizes).toEqual(files.map((file) => file.size));
});

test("retain a pushed build once a release locks it, past its lease", async () => {
    // push a build, then release it as the owner signed in again
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    await forge.push(build.reader);
    const manifest = await BuildStore.digest(build.manifest, forge.environment.crypto);
    await forge.signOwnerIn();
    const released = await forge.publish(build.reader);

    // wake past its lease
    await forge.simulation.jump(forge.environment.clock.now() + UNRELEASED_MILLISECONDS + 1000);

    // read and verify every file of the released build after the collection
    const store = await forge.store();
    const files = await (await store.contents(manifest)).distributed();
    const sizes = await Promise.all(files.map(async (file) => (await store.file(file)).length));
    expect({ released: released.manifest, sizes }).toEqual({
        released: manifest,
        sizes: files.map((file) => file.size),
    });
});

test("serve a release's npm units and its Bun output by digest to its readers only, and retain them past their leases", async () => {
    // keep a build
    const forge = await TestForge.open();
    const { environment } = forge.home;
    const { crypto } = environment;
    const encoder = new TextEncoder();
    const output = {
        runtime: "bun" as const,
        emit: true,
        workloads: {},
        views: {},
        directory: "bun",
    };
    const unit = await MemoryBuild.write(
        new Map([
            ["package.json", encoder.encode(JSON.stringify(TINY))],
            ["bun/index.js", encoder.encode("export const tiny = 1;\n")],
            ["bun/index.d.ts", encoder.encode("export declare const tiny: number;\n")],
        ]),
        {
            role: "app",
            package: TINY,
            outputs: { bun: { ...output, exports: {}, peers: {} } },
        },
        crypto,
    );
    const unitManifest = await BuildStore.digest(unit.manifest, crypto);
    const build = await MemoryBuild.of(CLOSURE, [], crypto, {
        files: new Map([["bun/index.js", encoder.encode('export { tiny } from "tiny";\n')]]),
        outputs: {
            bun: {
                ...output,
                exports: { ".": { path: "bun/index.js", imports: ["tiny"] } },
                peers: {},
            },
        },
        requires: { tiny: TINY.version },
    });
    const buildManifest = await BuildStore.digest(build.manifest, crypto);

    // publish the build locked with its unit
    const published = await forge.publish(build, "private", [unit]);
    await using copied = await TestBuildStore.create(environment);
    const builds = `${forge.origin}${forgePackage.mounts.builds.path}`;
    const root = { kind: "lock", digest: published.lock } as const;
    const source = new PackageClient(
        `${builds}/${BuildObject.path(root)}/`,
        forge.endpoint(forge.owner).fetch,
    );
    await copied.pull(source, root, ["bun"]);

    // refuse the build to a stranger
    const stranger = await forge.signIn(STRANGER_EMAIL);
    const refused = await as(forge, stranger)(
        new URL(`${builds}/${buildManifest}/manifest.json`),
        {},
    );
    await refused.body?.cancel();

    // retain both past their leases
    const store = await forge.store();
    await forge.simulation.jump(forge.environment.clock.now() + UNRELEASED_MILLISECONDS + 1000);
    expect({
        copied: [
            await copied.containsOutput(unitManifest, "bun"),
            await copied.containsOutput(buildManifest, "bun"),
        ],
        refused: refused.status,
        kept: [await store.contains(unitManifest), await store.contains(buildManifest)],
    }).toEqual({
        copied: [true, true],
        refused: 404,
        kept: [true, true],
    });
});

/** Fetch from a forge as a person with their token, or anonymously. */
function as(forge: TestForge, person?: TestPerson) {
    return (input: URL, options: RequestInit) =>
        forge.endpoint(person).fetch(new Request(input, options));
}
