import { auditedCalls } from "@destack/object/test";
import type { TestPerson } from "@destack/account/test";
import { ACCOUNT_ID, TestForge } from "../src/test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { Commit, Digest, found, Identifier, present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { Package, PackageId } from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { BuildStore } from "@destack/build/store";
import { MemoryBuild } from "@destack/package/test";
import { Registry } from "@destack/registry/client";
import { LATEST_TAG, packageObject } from "@destack/registry/object";
import { fixtureBuild, prereleaseBuild, uncommittedBuild } from "./fixture/build.ts";
import { COMMIT } from "./fixture/package.ts";
import { Claim } from "@destack/object";
import { Account, account } from "@destack/account/object";
import { OPERATOR_EMAIL } from "@destack/account/test";
import { Snapshot } from "@destack/db";

/** Read the code a refused call fails with, or accepted. */
async function code(attempt: Promise<unknown>): Promise<string> {
    const refused = await refusal(attempt);

    return refused === "done" ? "accepted" : refused[0];
}

test("publish pushed builds as releases, replaying retried requests, refusing versions taken before and stable builds of uncommitted changes, and publishing prereleases of them", async () => {
    // publish a package, tagging it latest
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(forge.owner);

    // retry the publication, publish the version again, and publish the next one under a tag
    const created = {
        spaceId: forge.space,
        requestId: RequestId.create(forge.environment),
        parentId: answer.packageId,
        manifest: await forge.push((await fixtureBuild("answer", "2026.9.1")).reader),
        tag: "next",
    };
    const next = await owner.release.create(created);
    expect([
        [answer.release.version, answer.release.commit, answer.release.manifest],
        (await owner.release.create(created)).id,
        await refusal(
            owner.release.create({
                ...created,
                requestId: RequestId.create(forge.environment),
            }),
        ),
        (await owner.tag.list({ spaceId: forge.space })).items.map((entry) => [
            entry.name,
            entry.version,
        ]),
    ]).toEqual([
        ["2026.9.0", COMMIT, answer.manifest],
        next.id,
        ["CONFLICT", "VERSION_PUBLISHED", "package version was already published"],
        [
            ["latest", "2026.9.0"],
            ["next", "2026.9.1"],
        ],
    ]);

    // refuse a tag a range reads, a build nobody pushed, a build of uncommitted changes, another package's build, and a stranger
    const missing = Digest.parse("f".repeat(64));
    const dirty = await forge.push((await uncommittedBuild()).reader);
    const greeting = (await forge.publish((await fixtureBuild("greeting")).reader)).manifest;
    const attempt = (changes: Partial<typeof created>, user = forge.owner) =>
        refusal(
            forge.client(user).release.create({
                ...created,
                requestId: RequestId.create(forge.environment),
                ...changes,
            }),
        );
    expect([
        await attempt({ tag: "x" }),
        await attempt({ manifest: missing }),
        await attempt({ manifest: dirty }),
        await attempt({ manifest: greeting }),
        await attempt({}, forge.stranger()),
    ]).toEqual([
        ["BAD_REQUEST", "TAG_INVALID", "invalid distribution tag: x"],
        ["NOT_FOUND", "BUILD_NOT_PUSHED", `build ${missing} was not pushed to the forge`],
        [
            "CONFLICT",
            "UNCOMMITTED_CHANGES",
            `build ${dirty} compiled a working tree with uncommitted changes`,
        ],
        ["CONFLICT", "FOREIGN_BUILD", "the build belongs to another package"],
        ["NOT_FOUND", "NOT_FOUND", `package ${created.parentId} not found`],
    ]);

    // publish a dev prerelease of uncommitted changes, which records no commit
    const prerelease = await owner.release.create({
        ...created,
        requestId: RequestId.create(forge.environment),
        manifest: await forge.push((await prereleaseBuild()).reader),
        tag: "dev",
    });
    expect([prerelease.version, prerelease.commit]).toEqual(["2026.9.2-dev.1", null]);
});

test("resolve a published release by version and by the tag pointing at it, and open its build through the releases", async () => {
    // publish a release, which latest points at, and read releases as its owner
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(forge.owner);
    const releases = new Registry(async () => forge.registry(), forge.environment);

    // name the same manifest by version and by tag, open its build, and refuse a tag pointing nowhere
    const { packageId } = answer;
    const spaceId = forge.space;
    const reader = await releases.open(packageId, LATEST_TAG);
    expect([
        (await owner.release.resolve({ spaceId, parentId: packageId, release: "2026.9.0" }))
            .manifest,
        await releases.find(packageId, LATEST_TAG),
        await reader.distributed(),
        await refusal(owner.release.resolve({ spaceId, parentId: packageId, release: "next" })),
    ]).toEqual([
        answer.manifest,
        answer.manifest,
        await answer.reader.distributed(),
        ["NOT_FOUND", "RELEASE_NOT_FOUND", `${packageId} has no release next`],
    ]);
});

test("release a stored build under a tag once, point another tag at the release, and refuse another build at a released version", async () => {
    // keep each build in a store of its own
    const forge = await TestForge.open();
    const releases = new Registry(async () => forge.registry(), forge.environment);
    const { crypto } = forge.environment;
    const committed = (await fixtureBuild("answer", "2026.9.1")).reader;
    const uncommitted = (await uncommittedBuild()).reader;
    const manifest = await BuildStore.digest(committed.manifest, crypto);
    const other = await BuildStore.digest(uncommitted.manifest, crypto);
    const { id, name } = PackageManifest.package(committed.manifest);

    // release under next, point latest at the release, and refuse the uncommitted build at its version
    const outcomes = [
        await releases.release(stored(committed), manifest, ACCOUNT_ID, "next"),
        await releases.release(stored(committed), manifest, ACCOUNT_ID, LATEST_TAG),
        await refusal(releases.release(stored(uncommitted), other, ACCOUNT_ID, "next")),
    ];

    // resolve the release by version and by each tag
    expect([
        outcomes,
        await releases.find(id, "2026.9.1"),
        await releases.find(id, "next"),
        await releases.find(id, LATEST_TAG),
    ]).toEqual([
        [
            "released",
            "pointed",
            ["CONFLICT", "RELEASE_CONFLICT", `${name}@2026.9.1 is published from another build`],
        ],
        manifest,
        manifest,
        manifest,
    ]);
});

test("store a pushed build in the forge's store by digest as its release's manifest", async () => {
    // push and release the build
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const pushed = await (await forge.store()).contents(answer.manifest);

    // store the pushed build as the release's manifest
    expect([pushed.manifest, await pushed.distributed()]).toEqual([
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
            "CONFLICT",
            "VERSION_ORDER",
            "publish releases in version order: 2026.9.0 precedes 2026.9.1",
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
        ["2026.9.0", COMMIT, null, null],
        published.id,
        ["next", "2026.9.0", 1],
        tag.id,
        2,
        "CONFLICT",
        ["NOT_FOUND", "UNPUBLISHED_TARGET", "distribution tag target is not published"],
        ["BAD_REQUEST", "TAG_INVALID", "invalid distribution tag: v2"],
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
        ["package.create", "success"],
        ["release.create", "success"],
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
    expect(taken).toEqual(["CONFLICT", "NAME_TAKEN", "name is taken"]);
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
        (await resolveRelease("2026.9.0")).manifest,
        await refusal(resolveRelease("next")),
        await refusal(resolve(forge.stranger(), "@other/answer")),
        (await resolve(forge.owner, "@example/greeting")).name,
        await refusal(resolve(forge.stranger(), "@example/greeting")),
    ]).toEqual([
        [answer.packageId, "answer"],
        "2026.9.0",
        answer.manifest,
        ["NOT_FOUND", "RELEASE_NOT_FOUND", `${answer.packageId} has no release next`],
        ["NOT_FOUND", "PACKAGE_NOT_FOUND", "package not found"],
        "greeting",
        ["NOT_FOUND", "PACKAGE_NOT_FOUND", "package not found"],
    ]);
});

test("deprecate releases, and yank them from new installs while exact pins still resolve them", async () => {
    // publish two versions of a package, and a package depending on the first
    const forge = await TestForge.open();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const next = await forge.publish((await fixtureBuild("answer", "2026.9.1")).reader);
    await forge.publish((await fixtureBuild("greeting")).reader);
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
        ["NOT_FOUND", "RELEASE_NOT_FOUND", `${answer.packageId} has no release latest`],
    ]);

    // restore the first version, its tags left where they moved, and refuse taking a yanked version again
    expect([
        await yank("answer@2026.9.0", false),
        (await resolve("2026.9.0")).version,
        await tags(),
        await refusal(
            owner.release.create({
                spaceId: forge.space,
                requestId: RequestId.create(forge.environment),
                parentId: answer.packageId,
                manifest: next.manifest,
            }),
        ),
    ]).toEqual([
        false,
        "2026.9.0",
        ["greeting latest 2026.9.0"],
        ["CONFLICT", "VERSION_PUBLISHED", "package version was already published"],
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
            Package.parse({ id, name: `@${handle}/kernel`, version: "2026.9.0" }),
            [],
            crypto,
            { role: "builtin", commit: Commit.parse(COMMIT) },
        );
        const packageId = PackageManifest.package(build.manifest).id;
        await forge.client(person).package.create({
            spaceId: forge.space,
            accountId,
            id: packageId,
            requestId: RequestId.create(forge.environment),
            name: "kernel",
            visibility: "private",
        });
        const manifest = await forge.push(build, person);

        // release it as the manager
        return refusal(
            forge.client(person).release.create({
                spaceId: forge.space,
                requestId: RequestId.create(forge.environment),
                parentId: packageId,
                manifest,
            }),
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
            "BUILTIN_FOREIGN",
            "@tenant-1/kernel is a builtin, which only the platform's organization publishes",
        ],
    });
});

/** Store a build as the registry reads one: its contents, and no units. */
function stored(reader: BuildReader) {
    return { contents: async () => reader, units: async () => [] };
}
