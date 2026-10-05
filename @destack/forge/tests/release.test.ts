import { expect, refusal, test } from "@destack/test";
import { accessRelationship, accessRole } from "@destack/access";
import { account } from "@destack/account/object";
import { eq, sql, type SQL, type Table } from "@destack/db";
import { ids } from "@destack/host/test";
import { Scope } from "@destack/sync";
import { TEST_DIALECTS } from "@destack/db/test";
import { found, Identifier, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { PackageId } from "@destack/package";
import { packageObject, release } from "../src/object/index.ts";
import { PackageArchive } from "../src/pack/index.ts";
import {
    ACCOUNT_ID,
    COMMIT,
    fixtureBuild,
    PackageFixture,
    uncommittedBuild,
} from "./fixture/package.ts";

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
async function npm(forge: PackageFixture, path: string, user?: string): Promise<unknown> {
    const response = await fetch(`${forge.origin}/npm${path}`, {
        headers: user === undefined ? {} : { authorization: `Bearer ${user}` },
    });
    const body: unknown = await response.json();

    return response.ok ? body : [response.status, body];
}

test.each(TEST_DIALECTS)(
    "publish pushed builds as releases, replaying retried requests and refusing versions taken before on %s",
    async (dialect) => {
        // publish a package, tagging it latest, and pack its build as the release keeps it
        await using forge = await PackageFixture.open(dialect);
        const answer = await forge.publish("answer");
        const owner = forge.client("owner");
        const archive = await PackageArchive.pack(answer.build, COMMIT);

        // retry the publication, publish the version again, and publish the next one under a tag
        const created = {
            accountId: ACCOUNT_ID,
            requestId: RequestId.create(),
            parentId: answer.packageId,
            manifest: await forge.push(await fixtureBuild("answer", "2026.9.1")),
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

        // refuse a tag a range reads, a build nobody pushed, a build of uncommitted changes, another package's build, and a stranger
        const missing = "f".repeat(64);
        const dirty = await forge.push(await uncommittedBuild());
        const greeting = (await forge.publish("greeting")).manifest;
        const attempt = (changes: Partial<typeof created>, user = "owner") =>
            refusal(
                forge
                    .client(user)
                    .release.create({ ...created, requestId: RequestId.create(), ...changes }),
            );
        expect([
            await attempt({ tag: "x" }),
            await attempt({ manifest: missing }),
            await attempt({ manifest: dirty }),
            await attempt({ manifest: greeting }),
            await attempt({}, "stranger"),
        ]).toEqual([
            ["BAD_REQUEST", "invalid distribution tag: x"],
            ["NOT_FOUND", `build ${missing} was not pushed to the forge`],
            ["CONFLICT", `build ${dirty} compiled a working tree with uncommitted changes`],
            ["CONFLICT", "the build is another package's"],
            ["NOT_FOUND", `no scope ${ACCOUNT_ID}`],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "keep a pushed build in the forge's store by digest as its release's manifest on %s",
    async (dialect) => {
        // push and release the build
        await using forge = await PackageFixture.open(dialect);
        const answer = await forge.publish("answer");
        const pushed = await forge.forge.store.contents(answer.manifest);

        // keep the pushed build as the release's manifest
        expect([pushed.manifest, await pushed.reader.distributed()]).toEqual([
            answer.build.manifest,
            await answer.build.reader.distributed(),
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "publish releases in order, each planned from the latest release, on %s",
    async (dialect) => {
        // refuse a release planned from one the package never published and publish both in order
        await using forge = await PackageFixture.open(dialect);
        const next = await fixtureBuild("answer", "2026.9.1");
        const early = await refusal(forge.release("answer", next));
        const first = await forge.publish("answer");
        const second = await forge.release("answer", next);
        const owner = forge.client("owner");
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
        await using forge = await PackageFixture.open(dialect);
        const answer = await forge.publish("answer");
        const owner = forge.client("owner");
        const packageId = answer.packageId;

        // read the release, create a tag, replay its request, move it, refuse a stale move and an unpublished target, and delete it
        const created = {
            requestId: RequestId.create(),
            accountId: ACCOUNT_ID,
            parentId: packageId,
            name: "next",
            version: "2026.9.0",
        };
        const [published] = (await owner.release.list({ accountId: ACCOUNT_ID })).items;
        if (published === undefined) {
            throw new TypeError("the owner lists no release");
        }
        const tag = await owner.tag.create(created);
        const moved = {
            requestId: RequestId.create(),
            accountId: ACCOUNT_ID,
            id: tag.id,
            version: "2026.9.0",
        };
        expect([
            [published.version, published.commit, published.deprecation, published.unpublishedAt],
            (await owner.release.get({ accountId: ACCOUNT_ID, id: published.id })).id,
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
                forge.client("stranger").release.get({ accountId: ACCOUNT_ID, id: published.id }),
            ),
        ]).toEqual([
            ["2026.9.0", COMMIT, null, null],
            published.id,
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
        expect(await forge.audited()).toEqual([
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
        // publish a package and claim its name again under another identity
        await using forge = await PackageFixture.open(dialect);
        await forge.publish("answer");
        const taken = await refusal(
            forge.client("owner").package.create({
                accountId: ACCOUNT_ID,
                id: Identifier.create("package"),
                requestId: RequestId.create(),
                name: "answer",
                visibility: "public",
            }),
        );

        // resolve the latest tag, an exact version, a missing tag and a missing package
        const version = async (path: string) => {
            const read = await npm(forge, path, "owner");

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
    },
);

test.each(TEST_DIALECTS)(
    "deprecate releases and unpublish them as npm allows on %s",
    async (dialect) => {
        // publish two versions of a package, and a package depending on the first
        await using forge = await PackageFixture.open(dialect);
        const answer = await forge.publish("answer");
        await forge.publish("answer", { version: "2026.9.1" });
        const greeting = await forge.publish("greeting");
        const owner = forge.client("owner");
        const named = (entry: { readonly parentId: string }) =>
            entry.parentId === answer.packageId ? "answer" : "greeting";
        const releases = new Map(
            (await owner.release.list({ accountId: ACCOUNT_ID })).items.map((entry) => [
                `${named(entry)}@${entry.version}`,
                entry.id,
            ]),
        );
        const tags = async () =>
            (await owner.tag.list({ accountId: ACCOUNT_ID })).items
                .map((entry) => `${named(entry)} ${entry.name} ${entry.version}`)
                .toSorted();
        const deprecate = (key: string, message: string | null) =>
            owner.release.deprecate({
                accountId: ACCOUNT_ID,
                id: found(releases, key),
                requestId: RequestId.create(),
                message,
            });
        const unpublish = (key: string) =>
            refusal(
                owner.release.unpublish({
                    accountId: ACCOUNT_ID,
                    id: found(releases, key),
                    requestId: RequestId.create(),
                }),
            );

        // deprecate the first version for npm to report and withdraw the deprecation of the second
        const read = async (path: string) =>
            DOCUMENT.parse(await npm(forge, `/@example/answer${path}`, "owner"));
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
            await npm(forge, "/@example/answer/2026.9.1", "owner"),
        ]).toEqual([
            ["CONFLICT", "@example/greeting@2026.9.0 depends on this release"],
            "done",
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
                    manifest: greeting.manifest,
                }),
            ),
        ]).toEqual([
            "done",
            ["answer latest 2026.9.0"],
            ["CONFLICT", "package version was already published"],
        ]);

        // refuse unpublishing releases published 72 hours ago
        await forge.regional.database
            .update(release.table)
            .set({ createdAt: Date.now() - 72 * 60 * 60 * 1000 })
            .where(eq(release.table.id, found(releases, "answer@2026.9.0")));
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
        await using forge = await PackageFixture.open(dialect);
        await forge.publish("answer", { visibility: "public" });
        const greeting = await forge.publish("greeting", { visibility: "unlisted" });
        const owner = forge.client("owner");
        const reads = async (user?: string) => {
            const client = forge.client(user);
            const document = await npm(forge, "/@example/greeting", user);

            return [
                (await client.package.list({ accountId: ACCOUNT_ID })).items
                    .map((entry) => entry.name)
                    .toSorted(),
                await code(client.package.get({ accountId: ACCOUNT_ID, id: greeting.packageId })),
                Array.isArray(document) ? FAILURE.parse(document[1]).code : "accepted",
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
        const current = (await owner.package.list({ accountId: ACCOUNT_ID })).items.find(
            (entry) => entry.id === greeting.packageId,
        );
        if (current === undefined) {
            throw new TypeError("the owner lists no greeting package");
        }
        const change = {
            accountId: ACCOUNT_ID,
            id: greeting.packageId,
            visibility: "private",
        } as const;
        expect([
            await code(
                forge.client("stranger").package.update({
                    ...change,
                    requestId: RequestId.create(),
                }),
            ),
            (
                await owner.package.update({
                    ...change,
                    requestId: RequestId.create(),
                    revision: current.revision,
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

test.each(TEST_DIALECTS)(
    "drop an account's copied rows once it moves to another residency, refusing its former owner's publish here, on %s",
    async (dialect) => {
        // publish a package of the eu account, push its next build and move the account to the us
        await using forge = await PackageFixture.open(dialect);
        const answer = await forge.publish("answer");
        const next = await forge.push(await fixtureBuild("answer", "2026.9.1"));
        const owner = forge.accounts.user(ids.owner);
        const current = await owner.account.get({ scope: ids.owner, id: ACCOUNT_ID });
        await owner.account.update({
            scope: ids.owner,
            id: ACCOUNT_ID,
            requestId: RequestId.create(),
            revision: current.revision,
            residencyId: "us",
        });

        // drop the account's row, scope, roles and relationships from the forge's copies
        const database = forge.regional.database;
        const rows = async (table: Table, where: SQL) =>
            (
                await database
                    .select({ id: sql`1` })
                    .from(table)
                    .where(where)
            ).length;
        const copied = async () => [
            await rows(account.table, eq(account.table.id, ACCOUNT_ID)),
            await rows(Scope.table, eq(Scope.table.scope, ACCOUNT_ID)),
            await rows(accessRole, eq(accessRole.scope, ACCOUNT_ID)),
            await rows(accessRelationship, eq(accessRelationship.scope, ACCOUNT_ID)),
        ];
        await expect.poll(copied, { timeout: 5000 }).toEqual([0, 0, 0, 0]);

        // refuse the former owner's next release, as for any stranger to the account
        expect(
            await refusal(
                forge.client("owner").release.create({
                    accountId: ACCOUNT_ID,
                    requestId: RequestId.create(),
                    parentId: answer.packageId,
                    manifest: next,
                }),
            ),
        ).toEqual(["NOT_FOUND", `no scope ${ACCOUNT_ID}`]);
    },
);

test.each(TEST_DIALECTS)(
    "find a published package's account by its package id across the universe on %s",
    async (dialect) => {
        // publish a package, and find its account through the directory as the forge, by the package id alone
        await using forge = await PackageFixture.open(dialect);
        const answer = await forge.publish("answer");
        const directory = forge.forge.resolver.directory;
        expect([
            await packageObject.lookup(directory, "id", [answer.packageId]),
            await packageObject.lookup(directory, "id", [
                PackageId.parse(Identifier.create("package")),
            ]),
        ]).toEqual([packageObject.reference(ACCOUNT_ID, answer.packageId), undefined]);
    },
);
