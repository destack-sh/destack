import { auditedCalls } from "@destack/object/test";
import { type TestPerson, OPERATOR_EMAIL } from "@destack/account/test";
import { ACCOUNT_ID, TEST_COMMIT, TestForge } from "../src/test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { Digest, found, Identifier, present, Version } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { PackageVersion, PackageId } from "@destack/package";
import { BuildImage, PackageManifest } from "@destack/package/manifest";
import { TestBuildStore } from "@destack/build/test";
import { MemoryBuild } from "@destack/package/test";
import { RemoteBuildStore } from "@destack/build/store";
import registryPackage from "@destack/registry/package";
import { LATEST_TAG, packageObject } from "@destack/registry/object";
import { fixtureBuild } from "./fixture/build.ts";
import { UNRELEASED_VERSION } from "./fixture/package.ts";
import { Claim } from "@destack/object";
import { Account, account } from "@destack/account/object";

/** Read the code a refused call fails with, or accepted. */
async function code(attempt: Promise<unknown>): Promise<string> {
    const refused = await refusal(attempt);

    return refused === "done" ? "accepted" : refused[0];
}

test("publish pushed images as releases", async () => {
    // publish a package, tagging it latest
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(forge.owner);

    // publish the next version under a tag
    const created = {
        requestId: RequestId.create(forge.environment),
        parentId: answer.packageId,
        image: await forge.push((await fixtureBuild("answer", "2026.9.1")).reader),
        tag: "next",
        commit: TEST_COMMIT,
    };
    const next = await forge.release(created);

    // record each pushed image, and answer the release again for a replayed or repeated request
    expect([
        [answer.release.version, answer.release.commit, answer.release.image],
        [next.version, next.commit, next.image],
        (await forge.release(created)).id,
        (
            await forge.release({
                ...created,
                requestId: RequestId.create(forge.environment),
            })
        ).id,
        (await owner.tag.list({ spaceId: forge.space })).items.map((entry) => [
            entry.name,
            entry.version,
        ]),
    ]).toEqual([
        ["2026.9.0", TEST_COMMIT, answer.image],
        ["2026.9.1", TEST_COMMIT, created.image],
        next.id,
        next.id,
        [
            ["latest", "2026.9.0"],
            ["next", "2026.9.1"],
        ],
    ]);

    // refuse a tag a range reads
    const missing = Digest.parse("f".repeat(64));
    const attempt = (input: typeof created, user = forge.owner) =>
        refusal(
            forge.release(
                {
                    ...input,
                    requestId: RequestId.create(forge.environment),
                },
                user,
            ),
        );
    expect([
        await attempt({ ...created, tag: "x" }),
        await attempt({ ...created, image: missing }),
        await attempt(created, forge.stranger()),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "TAG_INVALID",
            "distribution tag 'x' is invalid: it must not be a version range",
        ],
        ["NOT_FOUND", "BUILD_NOT_FOUND", `image '${missing}' not found`],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission 'publish' is forbidden: it is not granted"],
    ]);
});

test("find a published release by version, tag, digest and id, and refuse its name to another package", async () => {
    // publish a release, which latest points at, and read releases as its owner
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(forge.owner);

    // name the same image by version and by tag
    const { packageId } = answer;
    const spaceId = forge.space;
    const resolve = async (release: string) =>
        (await owner.release.resolve({ spaceId, parentId: packageId, release })).image;
    const { url, fetch } = forge.registry();
    const builds = `${url}${registryPackage.mounts.builds.path}`;
    const reader = await RemoteBuildStore.open(
        builds,
        fetch,
        await resolve(LATEST_TAG),
        forge.environment.crypto,
    );
    expect([
        await resolve("2026.9.0"),
        await resolve(LATEST_TAG),
        await reader.references(),
        await refusal(owner.release.resolve({ spaceId, parentId: packageId, release: "next" })),
    ]).toEqual([
        answer.image,
        answer.image,
        await answer.reader.references(),
        ["NOT_FOUND", "RELEASE_NOT_FOUND", `release 'next' of '${packageId}' not found`],
    ]);

    // store the image of the build alone, and the build by its digest
    const store = await forge.store();
    const pushed = await store.contents(answer.manifest);
    expect([await store.image(answer.image), pushed.manifest, await pushed.references()]).toEqual([
        BuildImage.of(answer.manifest),
        answer.reader.manifest,
        await answer.reader.references(),
    ]);

    // find the package's account through the directory by the package id alone
    const claims = forge.claims;
    expect([
        await Claim.lookup(packageObject, claims, "id", [packageId]),
        await Claim.lookup(packageObject, claims, "id", [
            PackageId.parse(Identifier.create("package", forge.environment)),
        ]),
    ]).toEqual([{ scope: ACCOUNT_ID, id: packageId, state: "active" }, undefined]);

    // refuse the package's name under another identity
    const taken = await refusal(
        owner.package.create({
            spaceId,
            accountId: ACCOUNT_ID,
            id: Identifier.create("package", forge.environment),
            requestId: RequestId.create(forge.environment),
            name: "answer",
            visibility: "public",
            role: "app",
        }),
    );
    expect(taken).toEqual(["CONFLICT", "NAME_EXISTS", "'name' already exists"]);
});

test("release one image of a version under each tag, refusing another image of it", async () => {
    // store two builds of answer at one unreleased version with other files
    const forge = await TestForge.open();
    const owner = forge.client(forge.owner);
    const { crypto } = forge.environment;
    const { id, name } = PackageManifest.package((await fixtureBuild("answer")).reader.manifest);
    const version = PackageVersion.parse({ id, name, version: UNRELEASED_VERSION });
    await using store = await TestBuildStore.create(forge.environment);
    const pushImage = async (notes: string) => {
        const build = await MemoryBuild.of(version, [], crypto, {
            files: new Map([["notes.txt", new TextEncoder().encode(notes)]]),
        });

        return await store.putImage(BuildImage.of(await store.putIndexed(build)));
    };
    const released = await pushImage("released");
    const other = await pushImage("other");

    // release under next, then the same image under latest, and refuse the other image under next
    await forge.publishImage(store, released, "next", { commit: TEST_COMMIT });
    await forge.publishImage(store, released, LATEST_TAG, { commit: TEST_COMMIT });
    const refused = await refusal(
        forge.publishImage(store, other, "next", { commit: TEST_COMMIT }),
    );
    const resolve = async (release: string) =>
        (await owner.release.resolve({ spaceId: forge.space, parentId: id, release })).image;

    // keep one release of the released image, which both tags point at
    expect([
        refused,
        (await owner.release.list({ spaceId: forge.space })).items.map((entry) => [
            entry.parentId,
            entry.version,
            entry.image,
        ]),
        (await owner.tag.list({ spaceId: forge.space })).items
            .map((entry) => `${entry.name} ${entry.version}`)
            .toSorted(),
        await resolve(UNRELEASED_VERSION),
        await resolve("next"),
        await resolve(LATEST_TAG),
    ]).toEqual([
        [
            "CONFLICT",
            "RELEASE_EXISTS",
            `release '${UNRELEASED_VERSION}' of '${id}' already exists: another image published it`,
        ],
        [[id, UNRELEASED_VERSION, released]],
        [`${LATEST_TAG} ${UNRELEASED_VERSION}`, `next ${UNRELEASED_VERSION}`],
        released,
        released,
        released,
    ]);
});

test("refuse publishing a dev build as a release, even from a commit", async () => {
    // store a dev build of answer
    const forge = await TestForge.open();
    const { crypto } = forge.environment;
    const { id, name } = PackageManifest.package((await fixtureBuild("answer")).reader.manifest);
    const version = PackageVersion.parse({ id, name, version: Version.development });
    await using store = await TestBuildStore.create(forge.environment);
    const image = await store.putImage(
        BuildImage.of(await store.putIndexed(await MemoryBuild.of(version, [], crypto))),
    );

    // refuse it, which no release of the version follows
    expect(
        await refusal(forge.publishImage(store, image, LATEST_TAG, { commit: TEST_COMMIT })),
    ).toEqual([
        "BAD_REQUEST",
        "BUILD_INVALID",
        `image '${image}' is invalid: its root is a dev build, not a release`,
    ]);
});

test("keep a version's first release when another image of its build publishes", async () => {
    // image one build of answer with two builds of a library it depends on
    const forge = await TestForge.open();
    const owner = forge.client(forge.owner);
    const { crypto } = forge.environment;
    const { id, name } = PackageManifest.package((await fixtureBuild("answer")).reader.manifest);
    const version = PackageVersion.parse({ id, name, version: UNRELEASED_VERSION });
    const library = PackageVersion.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000f1",
        name: "@example/library",
        version: "2026.9.0",
    });
    await using store = await TestBuildStore.create(forge.environment);
    const root = await store.putIndexed(await MemoryBuild.of(version, [], crypto));
    const pushImage = async (notes: string) => {
        const files = new Map([["notes.txt", new TextEncoder().encode(notes)]]);
        const built = await store.putIndexed(await MemoryBuild.of(library, [], crypto, { files }));

        return await store.putImage(
            BuildImage.of(root, { [root]: { [library.name]: built }, [built]: {} }),
        );
    };
    const first = await pushImage("first");
    const second = await pushImage("second");

    // release both images at the version
    await forge.publishImage(store, first, LATEST_TAG, { commit: TEST_COMMIT });
    await forge.publishImage(store, second, LATEST_TAG, { commit: TEST_COMMIT });

    // keep the one release of the first image, beside the library it released
    expect([
        (await owner.release.list({ spaceId: forge.space, where: { parentId: id } })).items.map(
            (entry) => [entry.parentId, entry.version, entry.image],
        ),
        (
            await owner.release.resolve({
                spaceId: forge.space,
                parentId: id,
                release: UNRELEASED_VERSION,
            })
        ).image,
    ]).toEqual([[[id, UNRELEASED_VERSION, first]], first]);
});

test("publish an image's builds in dependency order", async () => {
    // image greeting with the answer build it requires in one store
    const forge = await TestForge.open();
    const owner = forge.client(forge.owner);
    const answer = (await fixtureBuild("answer")).reader;
    const greeting = (await fixtureBuild("greeting")).reader;
    await using store = await TestBuildStore.create(forge.environment);
    const root = await store.putIndexed(greeting);
    const image = await store.putImage(
        await BuildImage.hoisted(
            root,
            { "@example/answer": await store.putIndexed(answer) },
            (digest) => store.get(digest),
        ),
    );

    // publish greeting twice, with answer
    await forge.publishImage(store, image, LATEST_TAG, { commit: TEST_COMMIT });
    await forge.publishImage(store, image, LATEST_TAG, { commit: TEST_COMMIT });

    // record answer's release before greeting's, once each
    const answerId = PackageManifest.package(answer.manifest).id;
    const listed = await owner.release.list({ spaceId: forge.space });
    expect(
        listed.items.map((entry) => [
            entry.parentId === answerId ? "answer" : "greeting",
            entry.version,
        ]),
    ).toEqual([
        ["answer", "2026.9.0"],
        ["greeting", "2026.9.0"],
    ]);
});

test("publish each package at its publishConfig access", async () => {
    // image greeting, without an access, with answer, whose package file asks public access
    const forge = await TestForge.open();
    const owner = forge.client(forge.owner);
    const answer = (await fixtureBuild("answer")).reader;
    const greeting = (await fixtureBuild("greeting")).reader;
    await using store = await TestBuildStore.create(forge.environment);
    const root = await store.putIndexed(greeting);
    const image = await store.putImage(
        await BuildImage.hoisted(
            root,
            { "@example/answer": await store.putIndexed(answer) },
            (digest) => store.get(digest),
        ),
    );

    // publish both, asking no visibility
    await forge.publishImage(store, image, LATEST_TAG, { commit: TEST_COMMIT });

    // list answer as its package file asks, and greeting unlisted, the default
    const listed = await owner.package.list({ spaceId: forge.space, orderBy: { name: "asc" } });
    expect(listed.items.map((entry) => [entry.name, entry.visibility])).toEqual([
        ["answer", "public"],
        ["greeting", "unlisted"],
    ]);
});

test("point latest at each released version", async () => {
    // image greeting with answer
    const forge = await TestForge.open();
    const owner = forge.client(forge.owner);
    const answer = (await fixtureBuild("answer")).reader;
    const greeting = (await fixtureBuild("greeting")).reader;
    await using store = await TestBuildStore.create(forge.environment);
    const root = await store.putIndexed(greeting);
    const image = await store.putImage(
        await BuildImage.hoisted(
            root,
            { "@example/answer": await store.putIndexed(answer) },
            (digest) => store.get(digest),
        ),
    );

    // publish both under a stage tag twice, pointing latest too
    const options = { commit: TEST_COMMIT, tags: [LATEST_TAG] };
    await forge.publishImage(store, image, "local", options);
    await forge.publishImage(store, image, "local", options);

    // resolve the stage tag and latest of each to its released version
    const resolve = async (id: PackageId, release: string) =>
        (await owner.release.resolve({ spaceId: forge.space, parentId: id, release })).version;
    const tagged = await Promise.all(
        [answer, greeting].map(async (reader) => {
            const { id, version } = PackageManifest.package(reader.manifest);

            return {
                version,
                local: await resolve(id, "local"),
                latest: await resolve(id, LATEST_TAG),
            };
        }),
    );
    expect(tagged).toEqual(
        tagged.map(({ version }) => ({ version, local: version, latest: version })),
    );
});

test("release an image's unreleased package of the account before the package requiring it", async () => {
    // host answer's package without releasing it
    const forge = await TestForge.open();
    const answer = (await fixtureBuild("answer")).reader;
    const greeting = (await fixtureBuild("greeting")).reader;
    const owner = forge.client(forge.owner);
    await owner.package.create({
        spaceId: forge.space,
        accountId: ACCOUNT_ID,
        id: PackageManifest.package(answer.manifest).id,
        requestId: RequestId.create(forge.environment),
        name: "answer",
        role: answer.manifest.role,
    });

    // release answer first, then greeting requiring it
    await forge.publish(greeting, "private", [answer]);
    const listed = await owner.release.list({ spaceId: forge.space });
    expect(
        listed.items.map((entry) => [
            entry.parentId === PackageManifest.package(answer.manifest).id ? "answer" : "greeting",
            entry.version,
        ]),
    ).toEqual([
        ["answer", "2026.9.0"],
        ["greeting", "2026.9.0"],
    ]);
});

test("refuse an image pinning a yanked version of a hosted package", async () => {
    // publish answer, then yank its release
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(forge.owner);
    await owner.release.yank({
        spaceId: forge.space,
        id: answer.release.id,
        requestId: RequestId.create(forge.environment),
        isYanked: true,
    });

    // refuse greeting's release pinning the yanked version
    const greeting = (await fixtureBuild("greeting")).reader;
    expect(await refusal(forge.publish(greeting, "private", [answer.reader]))).toEqual([
        "PRECONDITION_FAILED",
        "DEPENDENCY_BLOCKED",
        "cannot release with dependency '@example/answer@2026.9.0': it is yanked",
    ]);
});

test("refuse an image pinning an unreleased version of another account's hosted package", async () => {
    // host a tenant's library without releasing it
    const forge = await TestForge.open();
    const { crypto } = forge.environment;
    const tenant = await forge.tenant();
    const library = PackageVersion.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000f2",
        name: "@tenant-1/library",
        version: "2026.9.0",
    });
    await forge.client(tenant.owner).package.create({
        spaceId: forge.space,
        accountId: tenant.accountId,
        id: library.id,
        requestId: RequestId.create(forge.environment),
        name: "library",
        role: "library",
    });

    // refuse an app of the publishing account pinning the library
    const app = PackageVersion.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000f3",
        name: "@example/app",
        version: "2026.9.0",
    });
    await using store = await TestBuildStore.create(forge.environment);
    const root = await store.putIndexed(await MemoryBuild.of(app, [], crypto));
    const built = await store.putIndexed(await MemoryBuild.of(library, [], crypto));
    const image = await store.putImage(
        BuildImage.of(root, { [root]: { [library.name]: built }, [built]: {} }),
    );
    expect(
        await refusal(forge.publishImage(store, image, LATEST_TAG, { commit: TEST_COMMIT })),
    ).toEqual([
        "PRECONDITION_FAILED",
        "DEPENDENCY_BLOCKED",
        "cannot release with dependency '@tenant-1/library@2026.9.0': it is not published",
    ]);
});

test("refuse publishing a version before the package's latest release", async () => {
    // publish the later release, then the earlier one
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer", "2026.9.1")).reader);
    const earlier = await refusal(forge.publish((await fixtureBuild("answer")).reader));

    // refuse the earlier version
    expect(earlier).toEqual([
        "PRECONDITION_FAILED",
        "VERSION_BLOCKED",
        "cannot publish release '2026.9.0': it precedes '2026.9.1', and releases publish in version order",
    ]);
});

test("publish a release declaring no terms, leaving its package's vocabulary empty", async () => {
    // publish a release of a build declaring no terms
    const forge = await TestForge.open();
    const published = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(forge.owner);

    // fix no terms in the package's vocabulary
    const read = await owner.package.get({ spaceId: forge.space, id: published.packageId });
    expect(read.vocabulary).toEqual({});
});

test("administer a package's releases and tags as its owner", async () => {
    // publish a package into the owner's account
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(forge.owner);
    const packageId = answer.packageId;

    // read the release, create a tag, replay its request, move it, refuse a stale move and a missing target, and delete it
    const created = {
        spaceId: forge.space,
        requestId: RequestId.create(forge.environment),
        parentId: packageId,
        name: "next",
        version: "2026.9.0",
    };
    const [published] = (await owner.release.list({ spaceId: forge.space })).items;
    if (published === undefined) {
        throw new TypeError("the owner lists no release");
    }
    const tag = await owner.tag.create(created);
    const moved = {
        spaceId: forge.space,
        requestId: RequestId.create(forge.environment),
        id: tag.id,
        version: "2026.9.0",
    };
    expect([
        [published.version, published.commit, published.deprecation, published.yankedAt],
        (await owner.release.get({ spaceId: forge.space, id: published.id })).id,
        [tag.name, tag.version, tag.revision],
        (await owner.tag.create(created)).id,
        (await owner.tag.update({ ...moved, revision: 1 })).revision,
        await code(
            owner.tag.update({
                ...moved,
                requestId: RequestId.create(forge.environment),
                revision: 1,
            }),
        ),
        await refusal(
            owner.tag.update({
                ...moved,
                requestId: RequestId.create(forge.environment),
                revision: 2,
                version: "2027.1.0",
            }),
        ),
        await refusal(
            owner.tag.create({
                ...created,
                requestId: RequestId.create(forge.environment),
                name: "v2",
            }),
        ),
        (await owner.tag.list({ spaceId: forge.space })).items.map((entry) => [
            entry.name,
            entry.version,
            entry.revision,
        ]),
        await owner.tag.delete({
            requestId: RequestId.create(forge.environment),
            spaceId: forge.space,
            id: tag.id,
            revision: 2,
        }),
        await code(
            forge.client(forge.stranger()).release.get({ spaceId: forge.space, id: published.id }),
        ),
    ]).toEqual([
        ["2026.9.0", TEST_COMMIT, null, null],
        published.id,
        ["next", "2026.9.0", 1],
        tag.id,
        2,
        "CONFLICT",
        ["NOT_FOUND", "TARGET_NOT_FOUND", "distribution tag target not found"],
        [
            "BAD_REQUEST",
            "TAG_INVALID",
            "distribution tag 'v2' is invalid: it must not be a version range",
        ],
        [
            ["latest", "2026.9.0", 1],
            ["next", "2026.9.0", 2],
        ],
        {},
        "NOT_FOUND",
    ]);

    // audit each caller's change once, failures too, never a nested call or a read
    await forge.simulation.idle();
    const forgeTypes = ["package.", "release.", "tag."];
    const audited = (await auditedCalls(forge.home.database))
        .filter((ended) => ended.execution.category === "activity")
        .filter((ended) => forgeTypes.some((prefix) => ended.method.startsWith(prefix)))
        .map((ended) => [ended.method, ended.execution.outcome.kind]);
    expect(audited).toEqual([
        ["package.publish", "success"],
        ["tag.create", "success"],
        ["tag.update", "success"],
        ["tag.update", "failure"],
        ["tag.update", "failure"],
        ["tag.create", "failure"],
        ["tag.delete", "success"],
    ]);
});

test("deprecate and yank releases", async () => {
    // publish two versions of a package, and a package depending on the first
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const next = await forge.publish((await fixtureBuild("answer", "2026.9.1")).reader);
    await forge.publish((await fixtureBuild("greeting")).reader, "private", [answer.reader]);
    const owner = forge.client(forge.owner);
    const named = (entry: { readonly parentId: string }) =>
        entry.parentId === answer.packageId ? "answer" : "greeting";
    const releases = new Map(
        (await owner.release.list({ spaceId: forge.space })).items.map((entry) => [
            `${named(entry)}@${entry.version}`,
            entry.id,
        ]),
    );
    const tags = async () =>
        (await owner.tag.list({ spaceId: forge.space })).items
            .map((entry) => `${named(entry)} ${entry.name} ${entry.version}`)
            .toSorted();
    const deprecate = (key: string, message: string | null) =>
        owner.release.deprecate({
            spaceId: forge.space,
            id: found(releases, key),
            requestId: RequestId.create(forge.environment),
            message,
        });
    const yank = async (key: string, isYanked: boolean) =>
        (
            await owner.release.yank({
                spaceId: forge.space,
                id: found(releases, key),
                requestId: RequestId.create(forge.environment),
                isYanked,
            })
        ).yankedAt !== null;

    // deprecate the first version and withdraw the deprecation of the second
    const resolve = (selected: string) =>
        owner.release.resolve({
            spaceId: forge.space,
            parentId: answer.packageId,
            release: selected,
        });
    const deprecated = await deprecate("answer@2026.9.0", "use 2026.9.1");
    await deprecate("answer@2026.9.1", "broken");
    await deprecate("answer@2026.9.1", null);
    expect([
        deprecated.deprecation,
        (await resolve("2026.9.0")).deprecation,
        (await resolve(LATEST_TAG)).deprecation,
    ]).toEqual(["use 2026.9.1", "use 2026.9.1", null]);

    // yank the dependency and the newest version, which takes latest off both
    expect([
        await yank("answer@2026.9.0", true),
        await yank("answer@2026.9.1", true),
        await yank("answer@2026.9.1", true),
        await tags(),
        await refusal(resolve(LATEST_TAG)),
    ]).toEqual([
        true,
        true,
        true,
        ["greeting latest 2026.9.0"],
        ["NOT_FOUND", "RELEASE_NOT_FOUND", `release 'latest' of '${answer.packageId}' not found`],
    ]);

    // restore the first version
    expect([
        await yank("answer@2026.9.0", false),
        (await resolve("2026.9.0")).version,
        await tags(),
        await refusal(
            forge.release({
                requestId: RequestId.create(forge.environment),
                parentId: answer.packageId,
                image: next.image,
                commit: TEST_COMMIT,
            }),
        ),
    ]).toEqual([
        false,
        "2026.9.0",
        ["greeting latest 2026.9.0"],
        [
            "CONFLICT",
            "RELEASE_EXISTS",
            `release '2026.9.1' of '${answer.packageId}' already exists: it is yanked`,
        ],
    ]);
});

test("show packages to readers by their visibility", async () => {
    // publish a public package and an unlisted one
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader, "public");
    const greeting = await forge.publish((await fixtureBuild("greeting")).reader, "unlisted");
    const owner = forge.client(forge.owner);
    const nameOf = (entry: { readonly parentId: string }) =>
        entry.parentId === answer.packageId ? "answer" : "greeting";
    const reads = async (user?: TestPerson) => {
        const client = forge.client(user);

        return [
            (
                await client.package.list({
                    spaceId: forge.space,
                    where: { accountId: ACCOUNT_ID },
                })
            ).items
                .map((entry) => entry.name)
                .toSorted(),
            await refusal(client.package.get({ spaceId: forge.space, id: greeting.packageId })),
            (await client.release.list({ spaceId: forge.space })).items
                .map((entry) => `${nameOf(entry)}@${entry.version}`)
                .toSorted(),
        ];
    };

    // let strangers and anonymous callers read both, listing only the public one
    const both = ["answer@2026.9.0", "greeting@2026.9.0"];
    expect([await reads(forge.owner), await reads(forge.stranger()), await reads()]).toEqual([
        [["answer", "greeting"], "done", both],
        [["answer"], "done", both],
        [["answer"], "done", both],
    ]);

    // forbid readers outside the account changing the package, and limit it to the account once private
    const current = (
        await owner.package.list({ spaceId: forge.space, where: { accountId: ACCOUNT_ID } })
    ).items.find((entry) => entry.id === greeting.packageId);
    if (current === undefined) {
        throw new TypeError("the owner lists no greeting package");
    }
    const change = {
        spaceId: forge.space,
        id: greeting.packageId,
        visibility: "private",
    } as const;
    expect([
        await refusal(
            forge.client(forge.stranger()).package.update({
                ...change,
                requestId: RequestId.create(forge.environment),
            }),
        ),
        (
            await owner.package.update({
                ...change,
                requestId: RequestId.create(forge.environment),
                revision: current.revision,
            })
        ).visibility,
        await reads(forge.owner),
        await reads(),
    ]).toEqual([
        ["FORBIDDEN", "PERMISSION_DENIED", "permission 'publish' is forbidden: it is not granted"],
        "private",
        [["answer", "greeting"], "done", both],
        [
            ["answer"],
            ["NOT_FOUND", "NOT_FOUND", `package '${greeting.packageId}' not found`],
            ["answer@2026.9.0"],
        ],
    ]);

    // resolve names and releases as the stranger, hiding the private package
    const spaceId = forge.space;
    const resolve = (user: TestPerson, name: string) =>
        forge.client(user).package.resolve({ spaceId, name });
    const stranger = forge.stranger();
    const resolveRelease = (named: string) =>
        forge.client(stranger).release.resolve({
            spaceId,
            parentId: answer.packageId,
            release: named,
        });

    // resolve the name, the release by tag and by version, and refuse missing names and releases
    const resolved = await resolve(stranger, "@example/answer");
    expect([
        [resolved.id, resolved.name],
        (await resolveRelease(LATEST_TAG)).version,
        (await resolveRelease("2026.9.0")).image,
        await refusal(resolveRelease("next")),
        await refusal(resolve(stranger, "@other/answer")),
        (await resolve(forge.owner, "@example/greeting")).name,
        await refusal(resolve(stranger, "@example/greeting")),
    ]).toEqual([
        [answer.packageId, "answer"],
        "2026.9.0",
        answer.image,
        ["NOT_FOUND", "RELEASE_NOT_FOUND", `release 'next' of '${answer.packageId}' not found`],
        ["NOT_FOUND", "PACKAGE_NOT_FOUND", "package not found"],
        "greeting",
        ["NOT_FOUND", "PACKAGE_NOT_FOUND", "package not found"],
    ]);
});

test("publish builtins under the platform's account alone", async () => {
    // find the account whose home the forge is in, which the operator manages, and a tenant's
    const forge = await TestForge.open();
    const { crypto } = forge.environment;
    const platformId = Account.ofHome(forge.space);
    const platformAccount = await forge.home.database
        .snapshot()
        .read(account.table)
        .findFirst({ where: { id: platformId } });
    const platformHandle = present(platformAccount, "the platform account").handle;
    const operator = forge.home.universe.person(OPERATOR_EMAIL);
    const tenant = await forge.tenant();

    // publish a builtin's package under each account and push its build as the account's manager
    const builtin = async (
        id: string,
        accountId: Identifier<"account">,
        handle: string,
        person: TestPerson,
    ) => {
        const build = await MemoryBuild.of(
            PackageVersion.parse({ id, name: `@${handle}/kernel`, version: "2026.9.0" }),
            [],
            crypto,
            { role: "builtin" },
        );
        const packageId = PackageManifest.package(build.manifest).id;
        const image = await forge.push(build, person);

        // release it as the manager
        return refusal(
            forge.release(
                {
                    requestId: RequestId.create(forge.environment),
                    parentId: packageId,
                    accountId,
                    image,
                    commit: TEST_COMMIT,
                },
                person,
            ),
        );
    };

    // release the platform's builtin, and refuse the tenant's
    expect({
        platform: await builtin(
            "package-01996ab0-0000-7000-8000-00000000c201",
            platformId,
            platformHandle,
            operator,
        ),
        tenant: await builtin(
            "package-01996ab0-0000-7000-8000-00000000c202",
            tenant.accountId,
            "tenant-1",
            tenant.owner,
        ),
    }).toEqual({
        platform: "done",
        tenant: [
            "FORBIDDEN",
            "BUILTIN_DENIED",
            "builtin '@tenant-1/kernel' is forbidden: only the platform's organization publishes builtins",
        ],
    });
});
