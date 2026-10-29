import objectPackage from "@destack/object/package";
import type { TestPerson } from "@destack/account/test";
import { expect, refusal, test } from "@destack/test";
import { encodeManifest, PackageStore } from "@destack/build/store";
import { TestPackageStore } from "@destack/build/test";
import { PackageManifest } from "@destack/package/manifest";
import { Commit, Digest } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { UNRELEASED_MILLISECONDS } from "../src/server/index.ts";
import { account } from "@destack/account/object";
import forgePackage from "../src/package.ts";
import { ACCOUNT_ID, TestForge } from "../src/test/index.ts";
import { fixtureBuild } from "./fixture/build.ts";
import { COMMIT } from "./fixture/package.ts";
import { MemoryBuild } from "@destack/package/test";
import { Package } from "@destack/package";
import { testHost } from "@destack/test/host";

/** The email a person without access to the account signs in with. */
const STRANGER_EMAIL = "stranger@example.test";

/** The npm release whose unit the closure package depends on. */
const TINY = { name: "tiny", version: "1.0.0", integrity: "sha512-dGlueQ==" };

/** The package whose build depends on the tiny npm unit. */
const CLOSURE = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000c105",
    name: "@example/closure",
    version: "2026.9.0",
});

test("serve a release's build by its digest to its package's readers only, yanked or not", async () => {
    // publish a private package, and copy its build by digest into another store as the owner
    const forge = await TestForge.open();
    const published = await forge.publish((await fixtureBuild("answer")).reader);
    const { environment } = forge.home;
    await using copied = await TestPackageStore.create(environment);
    const location = (digest: Digest) => ({
        manifest: digest,
        url: `${forge.origin}${forgePackage.adapters.builds.path}/${digest}/`,
    });
    await copied.copy(location(published.manifest), as(forge, forge.owner));

    // refuse anonymous and other callers and an unknown build, and serve the build once yanked
    const stranger = await forge.signIn(STRANGER_EMAIL);
    const status = async (digest: Digest, person?: TestPerson) => {
        const response = await as(forge, person)(
            new URL(`${location(digest).url}manifest.json`),
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

test("refuse uploads outside a package, and a manifest push from a caller who may not publish its package", async () => {
    // publish the package as its owner and push its next build's files as a stranger
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    const manifest = encodeManifest(PackageManifest.parse(build.manifest));
    const digest = await Digest.of(manifest, testHost().crypto);
    const stranger = await forge.signIn(STRANGER_EMAIL);

    // refuse the owner's manifest outside its package, and the stranger's below it as a missing package
    const builds = `${forge.origin}${forgePackage.adapters.builds.path}`;
    const put = (person: TestPerson, path: string) =>
        as(forge, person)(new URL(`${builds}/${path}manifest.json`), {
            method: "PUT",
            body: manifest,
            headers: { "content-type": "application/json" },
        });
    const outside = await put(forge.owner, `${digest}/`);
    const pushed = await put(stranger, `${PackageManifest.package(build.manifest).id}/${digest}/`);
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
                message: "uploads must target a package",
                reason: "UPLOAD_OUTSIDE_PACKAGE",
                domain: "@destack/forge",
            },
        ],
        pushed: [
            404,
            {
                defined: true,
                code: "NOT_FOUND",
                status: 404,
                message: "package not found",
                reason: "PACKAGE_NOT_FOUND",
                domain: "@destack/forge",
            },
        ],
        isStored: false,
    });
});

test("authorize a manifest push from the forge's replica of the account, following a publisher relation the account service grants", async () => {
    // publish the package as its owner, and store its next build to push as a stranger
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    const digest = await Digest.of(
        encodeManifest(PackageManifest.parse(build.manifest)),
        forge.environment.crypto,
    );
    const stranger = await forge.signIn(STRANGER_EMAIL);
    const push = async () =>
        refusal(
            forge
                .builds(PackageManifest.package(build.reader.manifest).id, stranger)
                .push(build.reader),
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
    await forge.settle();

    // refuse the stranger's first file upload, whichever file the concurrent uploads send first
    const uploads = `${forgePackage.adapters.builds.path}/${PackageManifest.package(build.manifest).id}/files`;
    const digests = (await build.reader.distributed()).map((file) => file.digest);
    const failed = digests.find(
        (each) => refused[2] === `cannot upload ${uploads}/${each}: HTTP 404`,
    );
    expect([refused, failed === undefined]).toEqual([
        ["BAD_GATEWAY", "UPLOAD_FAILED", `cannot upload ${uploads}/${failed}: HTTP 404`],
        false,
    ]);
    expect({
        accepted: await push(),
        isStored: await (await forge.store()).contains(digest),
    }).toEqual({
        accepted: "done",
        isStored: true,
    });
});

test("collect a pushed but unreleased build once its lease ends, leaving released builds and archives", async () => {
    // publish one build and push the next without releasing it
    const forge = await TestForge.open();
    const published = await forge.publish((await fixtureBuild("answer")).reader, "public");
    const unreleased = await forge.push((await fixtureBuild("answer", "2026.9.1")).reader);
    const store = await forge.store();
    const stored = async () => ({
        released: await store.contains(published.manifest),
        archive: await store.has(published.release.distribution.digest),
        unreleased: await store.contains(unreleased),
    });

    // retain the unreleased build within its lease, and collect it once the lease ends
    const { clock } = forge.environment;
    await forge.jump(clock.now());
    const within = await stored();
    await forge.jump(clock.now() + UNRELEASED_MILLISECONDS + 1000);
    expect({ within, after: await stored() }).toEqual({
        within: {
            released: true,
            archive: true,
            unreleased: true,
        },
        after: {
            released: true,
            archive: true,
            unreleased: false,
        },
    });

    // read and verify every file of the released build after the collection
    const files = await (await store.contents(published.manifest)).distributed();
    const sizes = await Promise.all(files.map(async (file) => (await store.file(file)).length));
    expect(sizes).toEqual(files.map((file) => file.size));
});

test("retain a pushed build once a release lists it, past its lease", async () => {
    // push a build, then release it as the owner signed in again
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    const manifest = await forge.push(build.reader);
    await forge.signOwnerIn();
    const released = await forge.publish(build.reader);

    // wake past its lease
    await forge.jump(forge.environment.clock.now() + UNRELEASED_MILLISECONDS + 1000);

    // read and verify every file of the released build after the collection
    const store = await forge.store();
    const files = await (await store.contents(manifest)).distributed();
    const sizes = await Promise.all(files.map(async (file) => (await store.file(file)).length));
    expect({ released: released.manifest, sizes }).toEqual({
        released: manifest,
        sizes: files.map((file) => file.size),
    });
});

test("serve a release's npm units and runtime builds by digest to its readers only, and retain them past their leases", async () => {
    // keep a build whose Bun output imports an npm unit, beside the unit, with their runtime builds
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
        { package: TINY, outputs: { bun: { ...output, exports: {}, dependencies: {} } } },
        crypto,
    );
    const unitManifest = await PackageStore.digest(unit.manifest, crypto);
    const dependency = {
        kind: "npm" as const,
        package: { name: TINY.name, version: TINY.version },
        registry: "https://registry.npmjs.org",
        integrity: TINY.integrity,
        manifest: unitManifest,
    };
    const build = await MemoryBuild.of(CLOSURE, [], crypto, {
        commit: Commit.parse(COMMIT),
        files: new Map([["bun/index.js", encoder.encode('export { tiny } from "tiny";\n')]]),
        outputs: {
            bun: {
                ...output,
                exports: { ".": { path: "bun/index.js", imports: ["tiny"] } },
                dependencies: { tiny: dependency },
            },
        },
    });
    await using published = await TestPackageStore.create(environment);
    await published.put(unit);
    const buildManifest = await published.put(build);
    const runtime = await published.attachRuntime(buildManifest, "bun");

    // publish the build with its unit and runtime builds, then copy the unit and the runtime build as the owner
    await forge.publish(build, "private", published);
    await using copied = await TestPackageStore.create(environment);
    const builds = `${forge.origin}${forgePackage.adapters.builds.path}`;
    const owner = as(forge, forge.owner);
    const referred = await owner(new URL(`${builds}/${buildManifest}/referrers/runtime%2Fbun`), {});
    const described: unknown = await referred.json();
    for (const manifest of [unitManifest, runtime]) {
        await copied.copy({ manifest, url: `${builds}/${manifest}/` }, owner);
    }

    // refuse the runtime build to a stranger
    const stranger = await forge.signIn(STRANGER_EMAIL);
    const refused = await as(forge, stranger)(new URL(`${builds}/${runtime}/manifest.json`), {});
    await refused.body?.cancel();

    // retain both past their leases, as the release lists them
    const store = await forge.store();
    await forge.jump(forge.environment.clock.now() + UNRELEASED_MILLISECONDS + 1000);
    expect({
        referred: described,
        copied: [await copied.contains(unitManifest), await copied.contains(runtime)],
        refused: refused.status,
        kept: [await store.contains(unitManifest), await store.contains(runtime)],
    }).toEqual({
        referred: { ...(await published.head(runtime)), path: "runtime/bun" },
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
