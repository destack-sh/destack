import { auditedCalls } from "@destack/object/test";
import type { TestPerson } from "@destack/account/test";
import { ACCOUNT_ID, TEST_COMMIT, TestForge } from "../src/test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { Digest, found, Identifier, present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { PackageVersion, PackageId } from "@destack/package";
import { Lock, PackageManifest } from "@destack/package/manifest";
import { TestBuildStore } from "@destack/build/test";
import { MemoryBuild } from "@destack/package/test";
import { Registry } from "@destack/registry/client";
import { LATEST_TAG, packageObject } from "@destack/registry/object";
import { fixtureBuild, prereleaseBuild } from "./fixture/build.ts";
import { PRERELEASE_VERSION } from "./fixture/package.ts";
import { Claim } from "@destack/object";
import { Account, account } from "@destack/account/object";
import { OPERATOR_EMAIL } from "@destack/account/test";
import { Snapshot } from "@destack/db";

/** Read the code a refused call fails with, or accepted. */
async function code(attempt: Promise<unknown>): Promise<string> {
    const refused = await refusal(attempt);

    return refused === "done" ? "accepted" : refused[0];
}

test("publish pushed locks as releases, answering retried and repeated releases, refusing stable releases without a commit", async () => {
    // publish a package, tagging it latest
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(forge.owner);

    // publish the next version under a tag
    const uncommitted = {
        requestId: RequestId.create(forge.environment),
        parentId: answer.packageId,
        name: "answer",
        lock: await forge.push((await fixtureBuild("answer", "2026.9.1")).reader),
        tag: "next",
    };
    const created = { ...uncommitted, commit: TEST_COMMIT };
    const next = await forge.release(created);
    expect([
        [answer.release.version, answer.release.commit, answer.release.lock],
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
        ["2026.9.0", TEST_COMMIT, answer.lock],
        next.id,
        next.id,
        [
            ["latest", "2026.9.0"],
            ["next", "2026.9.1"],
        ],
    ]);

    // refuse a tag a range reads
    const missing = Digest.parse("f".repeat(64));
    const answerBuild = (await fixtureBuild("answer")).reader;
    const greeting = await forge.publish((await fixtureBuild("greeting")).reader, "private", [
        answerBuild,
    ]);
    const attempt = (input: typeof uncommitted, user = forge.owner) =>
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
        await attempt({ ...created, lock: missing }),
        await attempt(uncommitted),
        await attempt({ ...created, lock: greeting.lock }),
        await attempt(created, forge.stranger()),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "TAG_INVALID",
            "distribution tag 'x' is invalid: it must not be a version range",
        ],
        ["NOT_FOUND", "BUILD_NOT_FOUND", `lock '${missing}' not found`],
        [
            "BAD_REQUEST",
            "BUILD_INVALID",
            `lock '${created.lock}' is invalid: a release of uncommitted changes must be a prerelease`,
        ],
        ["BAD_REQUEST", "BUILD_INVALID", "build is invalid: it belongs to another package"],
        [
            "FORBIDDEN",
            "PERMISSION_DENIED",
            "permission 'publish' is forbidden: the caller lacks it",
        ],
    ]);

    // publish a dev prerelease without a commit
    const prerelease = await forge.release({
        ...uncommitted,
        requestId: RequestId.create(forge.environment),
        lock: await forge.push((await prereleaseBuild()).reader),
        tag: "dev",
    });
    expect([prerelease.version, prerelease.commit]).toEqual([PRERELEASE_VERSION, null]);
});

test("resolve a published release by version and by the tag pointing at it, and open its build through the releases", async () => {
    // publish a release, which latest points at, and read releases as its owner
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(forge.owner);
    const releases = new Registry(async () => forge.registry(), forge.environment);

    // name the same lock by version and by tag
    const { packageId } = answer;
    const spaceId = forge.space;
    const reader = await releases.open(packageId, LATEST_TAG);
    expect([
        (await owner.release.resolve({ spaceId, parentId: packageId, release: "2026.9.0" })).lock,
        await releases.find(packageId, LATEST_TAG),
        await reader.distributed(),
        await refusal(owner.release.resolve({ spaceId, parentId: packageId, release: "next" })),
    ]).toEqual([
        answer.lock,
        answer.lock,
        await answer.reader.distributed(),
        ["NOT_FOUND", "RELEASE_NOT_FOUND", `release 'next' of '${packageId}' not found`],
    ]);
});

test("release a stored lock under a tag, answer the same release under another tag, and refuse another lock at its version", async () => {
    // store two prerelease builds of answer at one version with other files
    const forge = await TestForge.open();
    const releases = new Registry(async () => forge.registry(), forge.environment);
    const { crypto } = forge.environment;
    const { id, name } = PackageManifest.package((await fixtureBuild("answer")).reader.manifest);
    const version = PackageVersion.parse({ id, name, version: PRERELEASE_VERSION });
    await using store = await TestBuildStore.create(forge.environment);
    const locked = async (notes: string) => {
        const build = await MemoryBuild.of(version, [], crypto, {
            files: new Map([["notes.txt", new TextEncoder().encode(notes)]]),
        });

        return await store.putLock(Lock.of(await store.put(build)));
    };
    const released = await locked("released");
    const other = await locked("other");

    // release under next
    const first = await releases.release(store, [released], ACCOUNT_ID, "next");
    const again = await releases.release(store, [released], ACCOUNT_ID, LATEST_TAG);
    const refused = await refusal(releases.release(store, [other], ACCOUNT_ID, "next"));

    // answer one release
    expect([
        again,
        refused,
        await releases.find(id, PRERELEASE_VERSION),
        await releases.find(id, "next"),
        await releases.find(id, LATEST_TAG),
    ]).toEqual([
        first,
        [
            "CONFLICT",
            "RELEASE_EXISTS",
            `release '${PRERELEASE_VERSION}' of '${id}' already exists: another lock published it`,
        ],
        released,
        released,
        released,
    ]);
});

test("publish a lock's builds of its account in dependency order, answering the same releases again", async () => {
    // lock greeting with the answer build it requires in one store
    const forge = await TestForge.open();
    const releases = new Registry(async () => forge.registry(), forge.environment);
    const answer = (await fixtureBuild("answer")).reader;
    const greeting = (await fixtureBuild("greeting")).reader;
    await using store = await TestBuildStore.create(forge.environment);
    const root = await store.put(greeting);
    const lock = await store.putLock(
        await Lock.hoisted(root, { "@example/answer": await store.put(answer) }, (digest) =>
            store.get(digest),
        ),
    );

    // publish greeting
    const first = await releases.publish(store, lock, ACCOUNT_ID, LATEST_TAG, TEST_COMMIT);
    const again = await releases.publish(store, lock, ACCOUNT_ID, LATEST_TAG, TEST_COMMIT);

    expect([versionsOf(first), versionsOf(again)]).toEqual([
        ["@example/answer@2026.9.0", "@example/greeting@2026.9.0"],
        ["@example/answer@2026.9.0", "@example/greeting@2026.9.0"],
    ]);
});

test("refuse releasing a lock pinning a package the forge hosts at a version it has not released", async () => {
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
    });

    // refuse greeting's release while answer has no release
    expect(await refusal(forge.publish(greeting, "private", [answer]))).toEqual([
        "PRECONDITION_FAILED",
        "DEPENDENCY_BLOCKED",
        "cannot release with dependency '@example/answer@2026.9.0': it is not published",
    ]);
});

test("store a pushed lock and its root build in the forge's store by digest", async () => {
    // push and release the build
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const store = await forge.store();
    const pushed = await store.contents(answer.manifest);

    // store the lock of the build alone
    expect([await store.lock(answer.lock), pushed.manifest, await pushed.distributed()]).toEqual([
        Lock.of(answer.manifest),
        answer.reader.manifest,
        await answer.reader.distributed(),
    ]);
});

test("publish releases in version order", async () => {
    // publish the later release, then refuse the earlier one
    const forge = await TestForge.open();
    const later = await forge.publish((await fixtureBuild("answer", "2026.9.1")).reader);
    const earlier = await refusal(forge.publish((await fixtureBuild("answer")).reader));
    const owner = forge.client(forge.owner);
    const published = await owner.package.get({ spaceId: forge.space, id: later.packageId });

    // refuse the earlier version, and fix no terms the release does not declare
    expect([earlier, published.vocabulary]).toEqual([
        [
            "PRECONDITION_FAILED",
            "VERSION_BLOCKED",
            "cannot publish release '2026.9.0': it precedes '2026.9.1', and releases publish in version order",
        ],
        {},
    ]);
});

test("administer releases and distribution tags of a published package as its account's owner", async () => {
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
    const audited = (await auditedCalls(forge.served.database))
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

test("refuse a package name its account took before", async () => {
    // publish a package and claim its name again under another identity
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    const taken = await refusal(
        forge.client(forge.owner).package.create({
            spaceId: forge.space,
            accountId: ACCOUNT_ID,
            id: Identifier.create("package", forge.environment),
            requestId: RequestId.create(forge.environment),
            name: "answer",
            visibility: "public",
        }),
    );
    expect(taken).toEqual(["CONFLICT", "NAME_EXISTS", "'name' already exists"]);
});

test("resolve a package by its global name and its release by version or tag, hiding private packages from strangers", async () => {
    // publish a public release, which latest points at, and a private package
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader, "public");
    await forge.publish((await fixtureBuild("greeting")).reader);
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
    const resolved = await resolve(forge.stranger(), "@example/answer");
    expect([
        [resolved.id, resolved.name],
        (await resolveRelease(LATEST_TAG)).version,
        (await resolveRelease("2026.9.0")).lock,
        await refusal(resolveRelease("next")),
        await refusal(resolve(forge.stranger(), "@other/answer")),
        (await resolve(forge.owner, "@example/greeting")).name,
        await refusal(resolve(forge.stranger(), "@example/greeting")),
    ]).toEqual([
        [answer.packageId, "answer"],
        "2026.9.0",
        answer.lock,
        ["NOT_FOUND", "RELEASE_NOT_FOUND", `release 'next' of '${answer.packageId}' not found`],
        ["NOT_FOUND", "PACKAGE_NOT_FOUND", "package not found"],
        "greeting",
        ["NOT_FOUND", "PACKAGE_NOT_FOUND", "package not found"],
    ]);
});

test("deprecate releases, and yank them from new installs while exact versions still resolve them", async () => {
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
                name: "answer",
                lock: next.lock,
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

test("let anyone read public and unlisted packages, list only public ones, and show private ones to the account alone", async () => {
    // publish a public package and an unlisted one
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader, "public");
    const greeting = await forge.publish((await fixtureBuild("greeting")).reader, "unlisted");
    const owner = forge.client(forge.owner);
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
            await code(client.package.get({ spaceId: forge.space, id: greeting.packageId })),
            (await client.release.list({ spaceId: forge.space })).items.length,
        ];
    };

    // let strangers and anonymous callers read both, listing only the public one
    expect([await reads(forge.owner), await reads(forge.stranger()), await reads()]).toEqual([
        [["answer", "greeting"], "accepted", 2],
        [["answer"], "accepted", 2],
        [["answer"], "accepted", 2],
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
        await code(
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
        "FORBIDDEN",
        "private",
        [["answer", "greeting"], "accepted", 2],
        [["answer"], "NOT_FOUND", 1],
    ]);
});

test("find a published package's account by its package id across the universe", async () => {
    // publish a package, and find its account through the directory as the forge, by the package id alone
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const claims = forge.claims;
    expect([
        await Claim.lookup(packageObject, claims, "id", [answer.packageId]),
        await Claim.lookup(packageObject, claims, "id", [
            PackageId.parse(Identifier.create("package", forge.environment)),
        ]),
    ]).toEqual([{ scope: ACCOUNT_ID, id: answer.packageId }, undefined]);
});

test("publish a builtin under the platform's organization, which owns the forge's home, and refuse one under another account", async () => {
    // find the account whose home the forge is in, which the operator manages, and a tenant's
    const forge = await TestForge.open();
    const { crypto } = forge.environment;
    const platformId = Account.ofHome(forge.space);
    const platformAccount = await Snapshot.live(forge.served.database)
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
        const lock = await forge.push(build, person);

        // release it as the manager
        return refusal(
            forge.release(
                {
                    requestId: RequestId.create(forge.environment),
                    parentId: packageId,
                    name: "kernel",
                    accountId,
                    lock,
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

/** List releases as their package names and versions. */
function versionsOf(published: readonly { readonly package: PackageVersion }[]): string[] {
    return published.map((release) => `${release.package.name}@${release.package.version}`);
}
