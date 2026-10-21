import objectPackage from "@destack/object/package";
import accountPackage from "@destack/account/package";
import { expect, refusal, test } from "@destack/test";
import { Clock } from "@destack/environment";
import { join } from "node:path";
import { LocalBucket } from "@destack/bucket/bun";
import { encodeManifest, PackageStore } from "@destack/build/store";
import { PackageManifest } from "@destack/package/manifest";
import { Digest } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { UNRELEASED_MILLISECONDS } from "../src/server/index.ts";
import { SWEEP_INTERVAL_MILLISECONDS } from "../src/server/sweep-controller.ts";
import { principal } from "@destack/access";
import { ids } from "@destack/account/test";
import { packageObject } from "../src/object/index.ts";
import forgePackage from "../src/package.ts";
import { fixtureBuild } from "./fixture/build.ts";
import { ACCOUNT_ID, openForge } from "./fixture/package.ts";

test("serve a release's build by its digest to its package's readers only, yanked or not", async () => {
    // publish a private package, and copy its build by digest into another store as the owner
    await using forge = await openForge();
    const published = await forge.publish((await fixtureBuild("answer")).reader);
    await using bucket = await LocalBucket.open(
        join(forge.state.directory, "copy"),
        "host-test",
        Clock.system,
    );
    const copied = new PackageStore(bucket);
    const location = (digest: string) => ({
        manifest: digest,
        url: `${forge.origin}/builds/${digest}/`,
    });
    await copied.copy(location(published.manifest), as(ids.owner));

    // refuse anonymous and other callers and an unknown build, and serve the build once yanked
    const status = async (digest: string, user?: string) =>
        (await as(user)(new URL(`${location(digest).url}manifest.json`), {})).status;
    const refused = [
        await status(published.manifest),
        await status(published.manifest, ids.stranger),
        await status("0".repeat(64), ids.owner),
    ];
    await forge.client(ids.owner).release.yank({
        spaceId: forge.space,
        id: published.release.id,
        requestId: RequestId.create(Date.now()),
        isYanked: true,
    });
    expect({
        copied: (await copied.contents(published.manifest)).manifest,
        refused,
        yanked: await status(published.manifest, ids.owner),
    }).toEqual({
        copied: published.reader.manifest,
        refused: [404, 404, 404],
        yanked: 200,
    });
});

test("refuse uploads outside a package, and a manifest push from a caller who may not publish its package", async () => {
    // publish the package as its owner and push its next build's files as a stranger
    await using forge = await openForge();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    const manifest = encodeManifest(PackageManifest.parse(build.manifest));
    const digest = await Digest.of(manifest);

    // refuse the owner's manifest outside its package, and the stranger's below it as a missing package
    const put = (user: string, path: string) =>
        as(user)(new URL(`${forge.origin}/builds/${path}manifest.json`), {
            method: "PUT",
            body: manifest,
            headers: { "content-type": "application/json" },
        });
    const outside = await put(ids.owner, `${digest}/`);
    const pushed = await put(
        ids.stranger,
        `${PackageManifest.package(build.manifest).id}/${digest}/`,
    );
    expect({
        outside: [outside.status, await outside.json()],
        pushed: [pushed.status, await pushed.json()],
        isStored: await forge.store.contains(digest),
    }).toEqual({
        outside: [404, { error: "NOT_FOUND", message: "upload below a package" }],
        pushed: [404, { error: "NOT_FOUND", message: "package not found" }],
        isStored: false,
    });
});

test("authorize a manifest push from the forge's replica of the account, following a role the account service grants", async () => {
    // publish the package as its owner, and store its next build to push as a stranger
    await using forge = await openForge();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    const digest = await Digest.of(encodeManifest(PackageManifest.parse(build.manifest)));
    const push = async () =>
        refusal(
            forge
                .builds(ids.stranger, PackageManifest.package(build.reader.manifest).id)
                .push(build.reader),
        );

    // refuse the stranger, grant them publishing at the account service and push once the replica follows
    const refused = await push();
    const owner = forge.accounts.user(ids.owner, accountPackage);
    const access = forge.accounts.user(ids.owner, objectPackage);
    const role = await access.role.create({
        scope: ACCOUNT_ID,
        requestId: RequestId.create(Date.now()),
        name: "publisher",
        description: "Publishes the account's packages",
        permissions: [packageObject.permission("publish")],
    });
    await owner.account.grant({
        scope: ids.owner,
        id: ACCOUNT_ID,
        requestId: RequestId.create(Date.now()),
        role: role.id,
        subject: principal.user.reference(Scope.universe.id, ids.stranger),
    });
    await forge.settle();

    // refuse the stranger's first file upload, whichever file the concurrent uploads send first
    const uploads = `${forgePackage.adapters.builds.path}/${PackageManifest.package(build.manifest).id}/files`;
    const files = await build.reader.distributed();
    const refusals = files.map((file) => [
        "BAD_GATEWAY",
        `cannot upload ${uploads}/${file.digest}: HTTP 404`,
    ]);
    expect(refusals).toContainEqual(refused);
    expect({ accepted: await push(), isStored: await forge.store.contains(digest) }).toEqual({
        accepted: "done",
        isStored: true,
    });
});

test("sweep a pushed but unreleased build once the window passes, leaving released builds and archives", async () => {
    // publish one build and push the next without releasing it
    await using forge = await openForge();
    const published = await forge.publish((await fixtureBuild("answer")).reader, "public");
    const unreleased = await forge.push((await fixtureBuild("answer", "2026.9.1")).reader);
    const store = forge.store;
    const stored = async () => ({
        released: await store.contains(published.manifest),
        archive: (await store.bucket.head(`files/${published.release.distribution.digest}`))?.size,
        unreleased: await store.contains(unreleased),
    });

    // leave the unreleased build within the window, mark it after and drop it an interval later
    await forge.sweep(Date.now());
    const within = await stored();
    const after = Date.now() + UNRELEASED_MILLISECONDS;
    await forge.sweep(after);
    await forge.sweep(after + SWEEP_INTERVAL_MILLISECONDS);
    expect({ within, after: await stored() }).toEqual({
        within: {
            released: true,
            archive: published.release.distribution.size,
            unreleased: true,
        },
        after: {
            released: true,
            archive: published.release.distribution.size,
            unreleased: false,
        },
    });

    // read and verify every file of the released build after the sweep
    const files = await (await store.contents(published.manifest)).distributed();
    const sizes = await Promise.all(files.map(async (file) => (await store.file(file)).length));
    expect(sizes).toEqual(files.map((file) => file.size));
});

test("retain the build of a release created after a sweep marked it through the next sweep", async () => {
    // push a build, and mark it in a sweep after the window
    await using forge = await openForge();
    await forge.publish((await fixtureBuild("answer")).reader);
    const build = await fixtureBuild("answer", "2026.9.1");
    const manifest = await forge.push(build.reader);
    const after = Date.now() + UNRELEASED_MILLISECONDS + 1000;
    await forge.sweep(after);

    // release the marked build and sweep an interval later
    const released = await forge.publish(build.reader);
    await forge.sweep(after + SWEEP_INTERVAL_MILLISECONDS);

    // read and verify every file of the released build after the sweep
    const store = forge.store;
    const files = await (await store.contents(manifest)).distributed();
    const sizes = await Promise.all(files.map(async (file) => (await store.file(file)).length));
    expect({ released: released.manifest, sizes }).toEqual({
        released: manifest,
        sizes: files.map((file) => file.size),
    });
});

/** Fetch as a user's bearer token, or anonymously. */
function as(user?: string) {
    return (input: URL, options: RequestInit) =>
        fetch(input, {
            ...options,
            headers: user === undefined ? {} : { authorization: `Bearer ${user}` },
        });
}
