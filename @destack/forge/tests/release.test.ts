import { auditedCalls } from "./fixture/audit.ts";
import { ids } from "@destack/account/test";
import type { ForgeFixture } from "../src/test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { found, Identifier, schema } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { PackageId } from "@destack/package";
import { Registry } from "../src/client/index.ts";
import { LATEST_TAG, packageObject } from "../src/object/index.ts";
import { PackageArchive } from "../src/pack/index.ts";
import { fixtureBuild, prereleaseBuild, uncommittedBuild } from "./fixture/build.ts";
import { ACCOUNT_ID, COMMIT, openForge } from "./fixture/package.ts";
import { Claim } from "@destack/object";

/** The fields of an npm version document the reads check. */
const DOCUMENT = schema
    .object({ version: schema.string(), deprecated: schema.string().exactOptional() })
    .strip();

/** The code of an npm failure body. */
const FAILURE = schema.object({ error: schema.string(), code: schema.string() });

/** Read the code a refused call fails with, or accepted. */
async function code(attempt: Promise<unknown>): Promise<string> {
    const refused = await refusal(attempt);

    return refused === "done" ? "accepted" : refused[0];
}

/** Read an npm document as a user, or anonymously: its body, or its status and failure. */
async function npm(forge: ForgeFixture, path: string, user?: string): Promise<unknown> {
    const response = await fetch(`${forge.origin}/npm${path}`, {
        headers: user === undefined ? {} : { authorization: `Bearer ${user}` },
    });
    const body: unknown = await response.json();

    return response.ok ? body : [response.status, body];
}

test("publish pushed builds as releases, replaying retried requests, refusing versions taken before and stable builds of uncommitted changes, and publishing prereleases of them", async () => {
    // publish a package, tagging it latest, and pack its build as the release stores it
    await using forge = await openForge();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(ids.owner);
    const archive = await PackageArchive.pack(answer.reader, COMMIT);

    // retry the publication, publish the version again, and publish the next one under a tag
    const created = {
        spaceId: forge.space,
        requestId: RequestId.create(Date.now()),
        parentId: answer.packageId,
        manifest: await forge.push((await fixtureBuild("answer", "2026.9.1")).reader),
        tag: "next",
    };
    const next = await owner.release.create(created);
    expect([
        [
            answer.release.version,
            answer.release.commit,
            answer.release.manifest,
            answer.release.distribution,
            answer.release.metadata,
        ],
        (await owner.release.create(created)).id,
        await refusal(
            owner.release.create({
                ...created,
                requestId: RequestId.create(Date.now()),
            }),
        ),
        (await owner.tag.list({ spaceId: forge.space })).items.map((entry) => [
            entry.name,
            entry.version,
        ]),
    ]).toEqual([
        ["2026.9.0", COMMIT, answer.manifest, archive.distribution, archive.metadata],
        next.id,
        ["CONFLICT", "package version was already published"],
        [
            ["latest", "2026.9.0"],
            ["next", "2026.9.1"],
        ],
    ]);

    // refuse a tag a range reads, a build nobody pushed, a build of uncommitted changes, another package's build, and a stranger
    const missing = "f".repeat(64);
    const dirty = await forge.push((await uncommittedBuild()).reader);
    const greeting = (await forge.publish((await fixtureBuild("greeting")).reader)).manifest;
    const attempt = (changes: Partial<typeof created>, user = ids.owner) =>
        refusal(
            forge.client(user).release.create({
                ...created,
                requestId: RequestId.create(Date.now()),
                ...changes,
            }),
        );
    expect([
        await attempt({ tag: "x" }),
        await attempt({ manifest: missing }),
        await attempt({ manifest: dirty }),
        await attempt({ manifest: greeting }),
        await attempt({}, ids.stranger),
    ]).toEqual([
        ["BAD_REQUEST", "invalid distribution tag: x"],
        ["NOT_FOUND", `build ${missing} was not pushed to the forge`],
        ["CONFLICT", `build ${dirty} compiled a working tree with uncommitted changes`],
        ["CONFLICT", "the build is another package's"],
        ["NOT_FOUND", `no package ${created.parentId}`],
    ]);

    // publish a dev prerelease of uncommitted changes, which records no commit
    const prerelease = await owner.release.create({
        ...created,
        requestId: RequestId.create(Date.now()),
        manifest: await forge.push((await prereleaseBuild()).reader),
        tag: "dev",
    });
    expect([prerelease.version, prerelease.commit, prerelease.metadata.gitHead]).toEqual([
        "2026.9.2-dev.1",
        null,
        undefined,
    ]);
});

test("resolve a published release by version and by the tag pointing at it, and open its build through the releases", async () => {
    // publish a release, which latest points at, and read releases as its owner
    await using forge = await openForge();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(ids.owner);
    const releases = new Registry(async () => ({
        url: forge.origin,
        space: forge.space,
        fetch: (request) => {
            const headers = new Headers(request.headers);
            headers.set("authorization", `Bearer ${ids.owner}`);

            return fetch(new Request(request, { headers }));
        },
    }));

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
        ["NOT_FOUND", `${packageId} lacks release next`],
    ]);
});

test("store a pushed build in the forge's store by digest as its release's manifest", async () => {
    // push and release the build
    await using forge = await openForge();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const pushed = await forge.store.contents(answer.manifest);

    // store the pushed build as the release's manifest
    expect([pushed.manifest, await pushed.distributed()]).toEqual([
        answer.reader.manifest,
        await answer.reader.distributed(),
    ]);
});

test("publish releases in order, each planned from the latest release,", async () => {
    // refuse a release planned from one the package never published and publish both in order
    await using forge = await openForge();
    const next = await fixtureBuild("answer", "2026.9.1");
    const early = await refusal(forge.publish(next.reader));
    const first = await forge.publish((await fixtureBuild("answer")).reader);
    const second = await forge.publish(next.reader);
    const owner = forge.client(ids.owner);
    const published = await owner.package.get({ spaceId: forge.space, id: first.packageId });

    // record the upgrade on the later release, and the terms neither release fixes
    expect([early, first.release.upgrade, second.release.upgrade, published.vocabulary]).toEqual([
        ["CONFLICT", "the first release plans no upgrade"],
        null,
        { from: "2026.9.0", steps: [] },
        {},
    ]);
});

test("administer releases and distribution tags of a published package as its account's owner", async () => {
    // publish a package into the owner's account
    await using forge = await openForge();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const owner = forge.client(ids.owner);
    const packageId = answer.packageId;

    // read the release, create a tag, replay its request, move it, refuse a stale move and a missing target, and delete it
    const created = {
        spaceId: forge.space,
        requestId: RequestId.create(Date.now()),
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
        requestId: RequestId.create(Date.now()),
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
                requestId: RequestId.create(Date.now()),
                revision: 1,
            }),
        ),
        await refusal(
            owner.tag.update({
                ...moved,
                requestId: RequestId.create(Date.now()),
                revision: 2,
                version: "2027.1.0",
            }),
        ),
        await refusal(
            owner.tag.create({
                ...created,
                requestId: RequestId.create(Date.now()),
                name: "v2",
            }),
        ),
        (await owner.tag.list({ spaceId: forge.space })).items.map((entry) => [
            entry.name,
            entry.version,
            entry.revision,
        ]),
        await owner.tag.delete({
            requestId: RequestId.create(Date.now()),
            spaceId: forge.space,
            id: tag.id,
            revision: 2,
        }),
        await code(
            forge.client(ids.stranger).release.get({ spaceId: forge.space, id: published.id }),
        ),
    ]).toEqual([
        ["2026.9.0", COMMIT, null, null],
        published.id,
        ["next", "2026.9.0", 1],
        tag.id,
        2,
        "CONFLICT",
        ["NOT_FOUND", "distribution tag target is unpublished"],
        ["BAD_REQUEST", "invalid distribution tag: v2"],
        [
            ["latest", "2026.9.0", 1],
            ["next", "2026.9.0", 2],
        ],
        {},
        "NOT_FOUND",
    ]);

    // audit each change once, the publication's vocabulary, tag and its pointing included, failures too, and never a read
    const audited = (await auditedCalls(forge.storage.database))
        .filter((ended) => ended.execution.category === "activity")
        .map((ended) => [ended.method, ended.execution.outcome.kind]);
    expect(audited).toEqual([
        ["package.create", "success"],
        ["package.advance", "success"],
        ["tag.create", "success"],
        ["package.point", "success"],
        ["release.create", "success"],
        ["tag.create", "success"],
        ["tag.update", "success"],
        ["tag.update", "failure"],
        ["tag.update", "failure"],
        ["tag.create", "failure"],
        ["tag.delete", "success"],
    ]);
});

test("enforce unique package names within an account and resolve global names to tagged and exact releases", async () => {
    // publish a package and claim its name again under another identity
    await using forge = await openForge();
    await forge.publish((await fixtureBuild("answer")).reader);
    const taken = await refusal(
        forge.client(ids.owner).package.create({
            spaceId: forge.space,
            accountId: ACCOUNT_ID,
            id: Identifier.create("package"),
            requestId: RequestId.create(Date.now()),
            name: "answer",
            visibility: "public",
        }),
    );

    // resolve the latest tag, an exact version, a missing tag and a missing package
    const version = async (path: string) => {
        const read = await npm(forge, path, ids.owner);

        return Array.isArray(read) ? read : DOCUMENT.parse(read).version;
    };
    expect([
        taken,
        await version("/@example/answer/latest"),
        await version("/@example/answer/2026.9.0"),
        await version("/@example/answer/next"),
        await version("/@other/answer"),
    ]).toEqual([
        ["CONFLICT", "name is taken"],
        "2026.9.0",
        "2026.9.0",
        [404, { error: "package version or tag not found", code: "NOT_FOUND" }],
        [404, { error: "package not found", code: "NOT_FOUND" }],
    ]);
});

test("resolve a package by its global name and its release by version or tag, hiding private packages from strangers", async () => {
    // publish a public release, which latest points at, and a private package
    await using forge = await openForge();
    const answer = await forge.publish((await fixtureBuild("answer")).reader, "public");
    await forge.publish((await fixtureBuild("greeting")).reader);
    const spaceId = forge.space;
    const resolve = (user: Identifier<"user">, name: string) =>
        forge.client(user).package.resolve({ spaceId, name });
    const resolveRelease = (named: string) =>
        forge.client(ids.stranger).release.resolve({
            spaceId,
            parentId: answer.packageId,
            release: named,
        });

    // resolve the name, the release by tag and by version, and refuse missing names and releases
    const resolved = await resolve(ids.stranger, "@example/answer");
    expect([
        [resolved.id, resolved.name],
        (await resolveRelease(LATEST_TAG)).version,
        (await resolveRelease("2026.9.0")).manifest,
        await refusal(resolveRelease("next")),
        await refusal(resolve(ids.stranger, "@other/answer")),
        (await resolve(ids.owner, "@example/greeting")).name,
        await refusal(resolve(ids.stranger, "@example/greeting")),
    ]).toEqual([
        [answer.packageId, "answer"],
        "2026.9.0",
        answer.manifest,
        ["NOT_FOUND", `${answer.packageId} lacks release next`],
        ["NOT_FOUND", "package not found"],
        "greeting",
        ["NOT_FOUND", "package not found"],
    ]);
});

test("deprecate releases, and yank them from new installs while exact pins still resolve them", async () => {
    // publish two versions of a package, and a package depending on the first
    await using forge = await openForge();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const next = await forge.publish((await fixtureBuild("answer", "2026.9.1")).reader);
    await forge.publish((await fixtureBuild("greeting")).reader);
    const owner = forge.client(ids.owner);
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
            requestId: RequestId.create(Date.now()),
            message,
        });
    const yank = async (key: string, isYanked: boolean) =>
        (
            await owner.release.yank({
                spaceId: forge.space,
                id: found(releases, key),
                requestId: RequestId.create(Date.now()),
                isYanked,
            })
        ).yankedAt !== null;

    // deprecate the first version for npm to report and withdraw the deprecation of the second
    const read = async (path: string) =>
        DOCUMENT.parse(await npm(forge, `/@example/answer${path}`, ids.owner));
    const deprecated = await deprecate("answer@2026.9.0", "use 2026.9.1");
    await deprecate("answer@2026.9.1", "broken");
    await deprecate("answer@2026.9.1", null);
    expect([
        deprecated.deprecation,
        (await read("/2026.9.0")).deprecated,
        (await read("/latest")).deprecated,
    ]).toEqual(["use 2026.9.1", "use 2026.9.1", undefined]);

    // yank the dependency and the newest version, moving latest back and leaving the archives for exact pins
    expect([
        await yank("answer@2026.9.0", true),
        await yank("answer@2026.9.1", true),
        await yank("answer@2026.9.1", true),
        await tags(),
        await npm(forge, "/@example/answer/latest", ids.owner),
        (
            await fetch(`${forge.origin}/npm/@example/answer/-/answer-2026.9.0.tgz`, {
                headers: { authorization: `Bearer ${ids.owner}` },
            })
        ).status,
    ]).toEqual([
        true,
        true,
        true,
        ["greeting latest 2026.9.0"],
        [404, { error: "package lacks releases to install", code: "NOT_FOUND" }],
        200,
    ]);

    // restore the first version, its tags left where they moved, and refuse taking a yanked version again
    expect([
        await yank("answer@2026.9.0", false),
        (await read("/2026.9.0")).version,
        await tags(),
        await refusal(
            owner.release.create({
                spaceId: forge.space,
                requestId: RequestId.create(Date.now()),
                parentId: answer.packageId,
                manifest: next.manifest,
            }),
        ),
    ]).toEqual([
        false,
        "2026.9.0",
        ["greeting latest 2026.9.0"],
        ["CONFLICT", "package version was already published"],
    ]);
});

test("let anyone read public and unlisted packages, list only public ones, and show private ones to the account alone", async () => {
    // publish a public package and an unlisted one
    await using forge = await openForge();
    await forge.publish((await fixtureBuild("answer")).reader, "public");
    const greeting = await forge.publish((await fixtureBuild("greeting")).reader, "unlisted");
    const owner = forge.client(ids.owner);
    const reads = async (user?: Identifier<"user">) => {
        const client = forge.client(user);
        const document = await npm(forge, "/@example/greeting", user);

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
            Array.isArray(document) ? FAILURE.parse(document[1]).code : "accepted",
            (await client.release.list({ spaceId: forge.space })).items.length,
        ];
    };

    // let strangers and anonymous callers read both, listing only the public one
    expect([await reads(ids.owner), await reads(ids.stranger), await reads()]).toEqual([
        [["answer", "greeting"], "accepted", "accepted", 2],
        [["answer"], "accepted", "accepted", 2],
        [["answer"], "accepted", "accepted", 2],
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
            forge.client(ids.stranger).package.update({
                ...change,
                requestId: RequestId.create(Date.now()),
            }),
        ),
        (
            await owner.package.update({
                ...change,
                requestId: RequestId.create(Date.now()),
                revision: current.revision,
            })
        ).visibility,
        await reads(ids.owner),
        await reads(),
    ]).toEqual([
        "FORBIDDEN",
        "private",
        [["answer", "greeting"], "accepted", "accepted", 2],
        [["answer"], "NOT_FOUND", "NOT_FOUND", 1],
    ]);
});

test("find a published package's account by its package id across the universe", async () => {
    // publish a package, and find its account through the directory as the forge, by the package id alone
    await using forge = await openForge();
    const answer = await forge.publish((await fixtureBuild("answer")).reader);
    const claims = forge.claims;
    expect([
        await Claim.lookup(packageObject, claims, "id", [answer.packageId]),
        await Claim.lookup(packageObject, claims, "id", [
            PackageId.parse(Identifier.create("package")),
        ]),
    ]).toEqual([{ scope: ACCOUNT_ID, id: answer.packageId }, undefined]);
});
