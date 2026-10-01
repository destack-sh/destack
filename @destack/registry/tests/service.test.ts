import { expect, test } from "@destack/test";
import { eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { RequestId } from "@destack/service/request";
import { release } from "../src/object/index.ts";
import { PackageArchive } from "../src/pack/index.ts";
import { ACCOUNT_ID, COMMIT, fixtureBuild, RegistryFixture } from "./registry.ts";

/** Read the code a refused call fails with, or accepted. */
function code(attempt: Promise<unknown>): Promise<string> {
    return attempt.then(
        () => "accepted",
        (error: { code: string }) => error.code,
    );
}

/** Read an npm document as a user, or anonymously: its body, or its status and failure. */
async function npm(registry: RegistryFixture, path: string, user?: string): Promise<unknown> {
    const response = await fetch(`${registry.origin}/npm${path}`, {
        headers: user === undefined ? {} : { authorization: `Bearer ${user}` },
    });
    const body: unknown = await response.json();

    return response.ok ? body : [response.status, body];
}

/** Read the code and message a refused call fails with, or accepted. */
function refusal(attempt: Promise<unknown>): Promise<string | [string, string]> {
    return attempt.then(
        () => "accepted",
        (error: { code: string; message: string }) => [error.code, error.message],
    );
}

test.each(TEST_DIALECTS)(
    "publish stored builds as releases, replaying retried requests and refusing versions taken before on %s",
    async (dialect) => {
        // publish a package, tagging it latest, and pack its build as the release keeps it
        await using registry = await RegistryFixture.open(dialect);
        const answer = await registry.publish("answer");
        const owner = registry.client("owner");
        const archive = await PackageArchive.pack(answer.build, COMMIT);

        // retry the publication, publish the version again, and publish the next one under a tag
        const created = {
            accountId: ACCOUNT_ID,
            requestId: RequestId.create(),
            parentId: answer.packageId,
            commit: COMMIT,
            manifest: await registry.server.store.put(await fixtureBuild("answer", "2026.9.1")),
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
            await refusal(owner.release.create({ ...created, requestId: RequestId.create() })),
            (await owner.tag.list({ accountId: ACCOUNT_ID })).items.map((entry) => [
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

        // refuse a tag a range reads, a build never stored, another package's build, and a stranger
        const greeting = await registry.server.store.put(await fixtureBuild("greeting"));
        const attempt = (changes: Record<string, string>, user = "owner") =>
            refusal(
                registry
                    .client(user)
                    .release.create({ ...created, requestId: RequestId.create(), ...changes }),
            );
        expect([
            await attempt({ tag: "x" }),
            await attempt({ manifest: "0".repeat(64) }),
            await attempt({ manifest: greeting }),
            await attempt({}, "stranger"),
        ]).toEqual([
            ["BAD_REQUEST", "invalid distribution tag: x"],
            ["NOT_FOUND", "package manifest not found"],
            ["CONFLICT", "the build is another package's"],
            ["NOT_FOUND", `no scope ${ACCOUNT_ID}`],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "publish releases in order, each planned from the latest release, on %s",
    async (dialect) => {
        // refuse a release planned from one the package never published, then publish both in order
        await using registry = await RegistryFixture.open(dialect);
        const next = await fixtureBuild("answer", "2026.9.1");
        const early = await refusal(registry.release("answer", next));
        const first = await registry.publish("answer");
        const second = await registry.release("answer", next);
        const owner = registry.client("owner");
        const published = await owner.package.get({ accountId: ACCOUNT_ID, id: first.packageId });

        // keep the upgrade on the later release, and the terms neither release fixes
        expect([
            early,
            first.release.upgrade,
            second.release.upgrade,
            published.vocabulary,
        ]).toEqual([
            ["CONFLICT", "the first release plans no upgrade"],
            null,
            { from: "2026.9.0", steps: [] },
            {},
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "administer releases and distribution tags of a published package as its account's owner on %s",
    async (dialect) => {
        // publish a package into the owner's account
        await using registry = await RegistryFixture.open(dialect);
        const answer = await registry.publish("answer");
        const owner = registry.client("owner");
        const packageId = answer.packageId;

        // read the release, create a tag, replay its request, move it, refuse a stale move and an unpublished target, and delete it
        const created = {
            requestId: RequestId.create(),
            accountId: ACCOUNT_ID,
            parentId: packageId,
            name: "next",
            version: "2026.9.0",
        };
        const [release] = (await owner.release.list({ accountId: ACCOUNT_ID })).items;
        const tag = await owner.tag.create(created);
        const moved = {
            requestId: RequestId.create(),
            accountId: ACCOUNT_ID,
            id: tag.id,
            version: "2026.9.0",
        };
        expect([
            [release!.version, release!.commit, release!.deprecation, release!.unpublishedAt],
            (await owner.release.get({ accountId: ACCOUNT_ID, id: release!.id })).id,
            [tag.name, tag.version, tag.revision],
            (await owner.tag.create(created)).id,
            (await owner.tag.update({ ...moved, revision: 1 })).revision,
            await code(owner.tag.update({ ...moved, requestId: RequestId.create(), revision: 1 })),
            await refusal(
                owner.tag.update({
                    ...moved,
                    requestId: RequestId.create(),
                    revision: 2,
                    version: "2027.1.0",
                }),
            ),
            await refusal(
                owner.tag.create({ ...created, requestId: RequestId.create(), name: "v2" }),
            ),
            (await owner.tag.list({ accountId: ACCOUNT_ID })).items.map((entry) => [
                entry.name,
                entry.version,
                entry.revision,
            ]),
            await owner.tag.delete({
                requestId: RequestId.create(),
                accountId: ACCOUNT_ID,
                id: tag.id,
                revision: 2,
            }),
            await code(
                registry.client("stranger").release.get({ accountId: ACCOUNT_ID, id: release!.id }),
            ),
        ]).toEqual([
            ["2026.9.0", COMMIT, null, null],
            release!.id,
            ["next", "2026.9.0", 1],
            tag.id,
            2,
            "CONFLICT",
            ["NOT_FOUND", "distribution tag target is not published"],
            ["BAD_REQUEST", "invalid distribution tag: v2"],
            [
                ["latest", "2026.9.0", 1],
                ["next", "2026.9.0", 2],
            ],
            {},
            "NOT_FOUND",
        ]);

        // audit each change once, the publication's vocabulary and tag included, failures too, and never a read
        expect(await registry.audited()).toEqual([
            ["package.create", "success"],
            ["package.advance", "success"],
            ["tag.point", "success"],
            ["release.create", "success"],
            ["tag.create", "success"],
            ["tag.update", "success"],
            ["tag.update", "failure"],
            ["tag.update", "failure"],
            ["tag.create", "failure"],
            ["tag.delete", "success"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "keep package names unique within an account and resolve global names to tagged and exact releases on %s",
    async (dialect) => {
        // publish a package, then claim its name again under another identity
        await using registry = await RegistryFixture.open(dialect);
        await registry.publish("answer");
        const taken = await refusal(
            registry.client("owner").package.create({
                accountId: ACCOUNT_ID,
                requestId: RequestId.create(),
                name: "answer",
                visibility: "public",
            }),
        );

        // resolve the latest tag, an exact version, a missing tag and a missing package
        const version = async (path: string) => {
            const read = await npm(registry, path, "owner");

            return Array.isArray(read) ? read : (read as { version: string }).version;
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
    },
);

test.each(TEST_DIALECTS)(
    "deprecate releases and unpublish them as npm allows on %s",
    async (dialect) => {
        // publish two versions of a package, and a package depending on the first
        await using registry = await RegistryFixture.open(dialect);
        const answer = await registry.publish("answer");
        await registry.publish("answer", { version: "2026.9.1" });
        const greeting = await registry.publish("greeting");
        const owner = registry.client("owner");
        const named = (entry: { readonly parentId: string }) =>
            entry.parentId === answer.packageId ? "answer" : "greeting";
        const ids = Object.fromEntries(
            (await owner.release.list({ accountId: ACCOUNT_ID })).items.map((entry) => [
                `${named(entry)}@${entry.version}`,
                entry.id,
            ]),
        );
        const tags = async () =>
            (await owner.tag.list({ accountId: ACCOUNT_ID })).items
                .map((entry) => `${named(entry)} ${entry.name} ${entry.version}`)
                .sort();
        const deprecate = (release: string, message: string | null) =>
            owner.release.deprecate({
                accountId: ACCOUNT_ID,
                id: ids[release]!,
                requestId: RequestId.create(),
                message,
            });
        const unpublish = (release: string) =>
            refusal(
                owner.release.unpublish({
                    accountId: ACCOUNT_ID,
                    id: ids[release]!,
                    requestId: RequestId.create(),
                }),
            );

        // deprecate the first version for npm to report and withdraw the deprecation of the second
        const read = async (path: string) =>
            (await npm(registry, `/@example/answer${path}`, "owner")) as {
                readonly version: string;
                readonly deprecated?: string;
            };
        const deprecated = await deprecate("answer@2026.9.0", "use 2026.9.1");
        await deprecate("answer@2026.9.1", "broken");
        await deprecate("answer@2026.9.1", null);
        expect([
            deprecated.deprecation,
            (await read("/2026.9.0")).deprecated,
            (await read("/latest")).deprecated,
        ]).toEqual(["use 2026.9.1", "use 2026.9.1", undefined]);

        // refuse unpublishing a dependency and unpublish the newest to move latest back
        expect([
            await unpublish("answer@2026.9.0"),
            await unpublish("answer@2026.9.1"),
            await unpublish("answer@2026.9.1"),
            (await read("/latest")).version,
            await read("/2026.9.1"),
        ]).toEqual([
            ["CONFLICT", "@example/greeting@2026.9.0 depends on this release"],
            "accepted",
            ["CONFLICT", "release is already unpublished"],
            "2026.9.0",
            [404, { error: "package version or tag not found", code: "NOT_FOUND" }],
        ]);

        // unpublish the dependent's only release to remove its tag and block its version forever
        expect([
            await unpublish("greeting@2026.9.0"),
            await tags(),
            await refusal(
                owner.release.create({
                    accountId: ACCOUNT_ID,
                    requestId: RequestId.create(),
                    parentId: greeting.packageId,
                    commit: COMMIT,
                    manifest: greeting.manifest,
                }),
            ),
        ]).toEqual([
            "accepted",
            ["answer latest 2026.9.0"],
            ["CONFLICT", "package version was already published"],
        ]);

        // refuse unpublishing releases published 72 hours ago
        await registry.regional.database
            .update(release.table)
            .set({ createdAt: Date.now() - 72 * 60 * 60 * 1000 })
            .where(eq(release.table.id, ids["answer@2026.9.0"]!));
        expect(await unpublish("answer@2026.9.0")).toEqual([
            "CONFLICT",
            "releases older than 72 hours stay published: deprecate them instead",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "let anyone read public and unlisted packages, list only public ones, and keep private ones to the account on %s",
    async (dialect) => {
        // publish a public package and an unlisted one
        await using registry = await RegistryFixture.open(dialect);
        await registry.publish("answer", { visibility: "public" });
        const greeting = await registry.publish("greeting", { visibility: "unlisted" });
        const owner = registry.client("owner");
        const reads = async (user?: string) => {
            const client = registry.client(user);
            const document = await npm(registry, "/@example/greeting", user);

            return [
                (await client.package.list({ accountId: ACCOUNT_ID })).items
                    .map((entry) => entry.name)
                    .sort(),
                await code(client.package.get({ accountId: ACCOUNT_ID, id: greeting.packageId })),
                Array.isArray(document) ? (document[1] as { code: string }).code : "accepted",
                (await client.release.list({ accountId: ACCOUNT_ID })).items.length,
            ];
        };

        // let strangers and anonymous callers read both, listing only the public one
        expect([await reads("owner"), await reads("stranger"), await reads()]).toEqual([
            [["answer", "greeting"], "accepted", "accepted", 2],
            [["answer"], "accepted", "accepted", 2],
            [["answer"], "accepted", "accepted", 2],
        ]);

        // forbid readers outside the account changing the package, and keep it to the account once private
        const [current] = (await owner.package.list({ accountId: ACCOUNT_ID })).items.filter(
            (entry) => entry.id === greeting.packageId,
        );
        const change = {
            accountId: ACCOUNT_ID,
            id: greeting.packageId,
            visibility: "private",
        } as const;
        expect([
            await code(
                registry.client("stranger").package.update({
                    ...change,
                    requestId: RequestId.create(),
                }),
            ),
            (
                await owner.package.update({
                    ...change,
                    requestId: RequestId.create(),
                    revision: current!.revision,
                })
            ).visibility,
            await reads("owner"),
            await reads(),
        ]).toEqual([
            "FORBIDDEN",
            "private",
            [["answer", "greeting"], "accepted", "accepted", 2],
            [["answer"], "NOT_FOUND", "NOT_FOUND", 1],
        ]);
    },
);
