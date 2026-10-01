import { expect, test } from "@destack/test";
import { Subject } from "@destack/sync";
import { reconciliation } from "@destack/service/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { principal } from "@destack/access";
import { Condition } from "@destack/db/query";
import { RequestId } from "@destack/service/request";
import { readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { claimTable } from "@destack/directory";
import { settlement } from "@destack/object";
import { eq } from "@destack/db";
import { reference } from "../src/object/index.ts";
import { History } from "./fixture/git.ts";
import { ids, RepositoryFixture } from "./fixture/repository.ts";

/** Report a call's outcome as its failure code, or as accepted. */
function outcome(call: Promise<unknown>): Promise<string> {
    return call.then(
        () => "accepted",
        (error: { code: string }) => error.code,
    );
}

test.each(TEST_DIALECTS)(
    "create, rename and refresh a platform repository as its account's owner, and refuse readers and strangers on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const owner = region.connect(ids.owner);
        const reader = region.connect(ids.reader);
        const stranger = region.connect(ids.stranger);
        const accountId = ids.account;

        // create a platform repository with its name in the directory and its storage in the region
        const created = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "site",
            hosting: "platform",
        });
        expect([
            created.scope,
            created.name,
            created.hosting,
            created.host,
            created.provider,
            created.providerRepositoryId,
            created.remote,
            created.defaultReference,
            created.authentication,
            created.connectedAccountId,
            created.secretSpaceId,
            created.secretId,
            created.revision,
        ]).toEqual([
            accountId,
            "site",
            "platform",
            null,
            "local",
            created.id,
            `file://${region.storage.directory}/${created.id}.git`,
            null,
            null,
            null,
            null,
            null,
            1,
        ]);
        const named = async () =>
            (await region.global.select().from(claimTable)).map((row) => [
                row.objectId,
                row.scope,
                row.key,
                row.state,
            ]);
        const site = [created.id, accountId, JSON.stringify([accountId, "site"]), "confirmed"];
        expect(await named()).toEqual([site]);

        // refuse a taken name and an origin mixing platform and remote fields and keep neither
        const refused = [
            await outcome(
                owner.repository.create({
                    accountId,
                    requestId: RequestId.create(),
                    name: "site",
                    hosting: "platform",
                }),
            ),
            await outcome(
                owner.repository.create({
                    accountId,
                    requestId: RequestId.create(),
                    name: "mixed",
                    hosting: "platform",
                    remote: "https://example.test/mixed.git",
                }),
            ),
        ];
        expect(refused).toEqual(["CONFLICT", "BAD_REQUEST"]);
        expect(await named()).toEqual([site]);
        expect(await readdir(region.storage.directory)).toEqual([`${created.id}.git`]);

        // let the reader read, forbid its writes, and keep strangers out
        expect([
            (await reader.repository.list({ accountId })).items.map((item) => item.name),
            await outcome(reader.repository.get({ accountId, id: created.id })),
            await outcome(
                reader.repository.create({
                    accountId,
                    requestId: RequestId.create(),
                    name: "other",
                    hosting: "platform",
                }),
            ),
            await outcome(
                reader.repository.refresh({
                    accountId,
                    id: created.id,
                    requestId: RequestId.create(),
                }),
            ),
            await outcome(stranger.repository.list({ accountId })),
            await outcome(stranger.repository.get({ accountId, id: created.id })),
        ]).toEqual([["site"], "accepted", "FORBIDDEN", "FORBIDDEN", "NOT_FOUND", "NOT_FOUND"]);

        // push a commit and an annotated tag, then observe them and the default branch on refresh
        const origin = fileURLToPath(created.remote!);
        const history = new History();
        history.tag("v1", history.commit("main", "README.md", "# Site\n"));
        const [commit, tag] = await history.write(origin);
        const refreshed = await owner.repository.refresh({
            accountId,
            id: created.id,
            requestId: RequestId.create(),
        });
        expect([refreshed.defaultReference, refreshed.revision]).toEqual(["refs/heads/main", 2]);
        const references = async () =>
            (
                await reader.reference.list({
                    accountId,
                    where: Condition.eq("parentId", created.id),
                    order: [{ column: "name", direction: "asc" }],
                })
            ).items.map((item) => [item.name, item.object, item.commit, item.deletedAt]);
        expect(await references()).toEqual([
            ["refs/heads/main", commit, commit, null],
            ["refs/tags/v1", tag, commit, null],
        ]);

        // rename the repository, moving its name in the directory
        const renamed = await owner.repository.update({
            accountId,
            id: created.id,
            requestId: RequestId.create(),
            revision: 2,
            name: "web",
        });
        expect([renamed.name, renamed.revision]).toEqual(["web", 3]);
        expect(await named()).toEqual([
            [created.id, accountId, JSON.stringify([accountId, "web"]), "confirmed"],
        ]);
        expect(
            await outcome(
                owner.repository.update({
                    accountId,
                    id: created.id,
                    requestId: RequestId.create(),
                    remote: "https://example.test/web.git",
                }),
            ),
        ).toBe("BAD_REQUEST");

        // record a moved branch and a deleted tag on the next refresh and keep unchanged rows
        history.commit("main", "README.md", "# Web\n");
        history.delete("refs/tags/v1");
        const [moved] = await history.write(origin);
        await owner.repository.refresh({
            accountId,
            id: created.id,
            requestId: RequestId.create(),
        });
        const [main, v1] = (
            await reader.reference.list({
                accountId,
                where: Condition.eq("parentId", created.id),
                order: [{ column: "name", direction: "asc" }],
            })
        ).items;
        expect([
            [main!.object, main!.commit, main!.deletedAt, main!.revision],
            [v1!.object, v1!.commit, v1!.deletedAt === null, v1!.revision],
        ]).toEqual([
            [moved, moved, null, 2],
            [tag, commit, false, 2],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "sync a repository's references to its readers and none to strangers on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const owner = region.connect(ids.owner);
        const accountId = ids.account;

        // observe one branch
        const created = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "notes",
            hosting: "platform",
        });
        const history = new History();
        history.commit("main", "notes.md", "first\n");
        const [commit] = await history.write(fileURLToPath(created.remote!));
        await owner.repository.refresh({
            accountId,
            id: created.id,
            requestId: RequestId.create(),
        });

        // follow the account's references as the reader and as a stranger
        const follow = async (user: string) => {
            const controller = new AbortController();
            const pages = await region
                .connect(user)
                .replica.sync(
                    { scope: accountId, queries: { references: { object: "reference" } } },
                    { signal: controller.signal },
                );
            const { value } = await pages[Symbol.asyncIterator]().next();
            controller.abort();

            return (value as { changes: { row: Record<string, unknown> }[] }).changes.map(
                (change) => [change.row.name, change.row.commit],
            );
        };
        expect(await follow(ids.reader)).toEqual([["refs/heads/main", commit]]);
        expect(
            await follow(ids.stranger).catch((error: { code: string; message: string }) => [
                error.code,
                error.message,
            ]),
        ).toEqual(["NOT_FOUND", `no scope ${accountId}`]);
    },
);

test.each(TEST_DIALECTS)(
    "delete and restore a repository through the trash, and purge its storage and name with it on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const owner = region.connect(ids.owner);
        const reader = region.connect(ids.reader);
        const accountId = ids.account;

        // keep a platform repository with one observed branch, and an anonymous Git remote
        const site = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "site",
            hosting: "platform",
        });
        const history = new History();
        history.commit("main", "README.md", "# Site\n");
        await history.write(fileURLToPath(site.remote!));
        await owner.repository.refresh({ accountId, id: site.id, requestId: RequestId.create() });
        const mirror = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "mirror",
            hosting: "git",
            remote: "https://git.example.test/mirror.git",
            authentication: "anonymous",
        });
        const state = async () => [
            (
                await owner.repository.list({
                    accountId,
                    order: [{ column: "name", direction: "asc" }],
                })
            ).items.map((item) => item.name),
            (await region.global.select().from(claimTable))
                .map((row) => (JSON.parse(row.key) as string[])[1])
                .sort(),
            await readdir(region.storage.directory),
            (await region.database.select().from(reference.table)).map((row) => row.name),
        ];

        // forbid the reader and move both repositories with their content to the trash
        expect(
            await outcome(
                reader.repository.delete({ accountId, id: site.id, requestId: RequestId.create() }),
            ),
        ).toBe("FORBIDDEN");
        for (const id of [site.id, mirror.id]) {
            await owner.repository.delete({ accountId, id, requestId: RequestId.create() });
        }
        expect(await state()).toEqual([
            [],
            ["mirror", "site"],
            [`${site.id}.git`],
            ["refs/heads/main"],
        ]);

        // refuse checkouts and refreshes of a repository in the trash
        expect([
            await outcome(owner.repository.access({ accountId, id: site.id, mode: "pull" })),
            await outcome(
                owner.repository.refresh({ accountId, id: site.id, requestId: RequestId.create() }),
            ),
        ]).toEqual(["CONFLICT", "CONFLICT"]);

        // restore the platform repository unchanged
        await owner.repository.restore({ accountId, id: site.id, requestId: RequestId.create() });
        expect(await state()).toEqual([
            ["site"],
            ["mirror", "site"],
            [`${site.id}.git`],
            ["refs/heads/main"],
        ]);

        // purge both, removing the platform repository's storage and references and both names
        await owner.repository.delete({ accountId, id: site.id, requestId: RequestId.create() });
        for (const id of [site.id, mirror.id]) {
            await owner.repository.purge({ accountId, id, requestId: RequestId.create() });
        }
        expect(await state()).toEqual([[], [], [], []]);
    },
);

test.each(TEST_DIALECTS)(
    "issue checkout access to pull and push, requiring the push permission and refusing origins the platform has no credential for on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const owner = region.connect(ids.owner);
        const reader = region.connect(ids.reader);
        const stranger = region.connect(ids.stranger);
        const accountId = ids.account;
        const [site, mirror, laptop] = [
            await owner.repository.create({
                accountId,
                requestId: RequestId.create(),
                name: "site",
                hosting: "platform",
            }),
            await owner.repository.create({
                accountId,
                requestId: RequestId.create(),
                name: "mirror",
                hosting: "git",
                remote: "https://git.example.test/mirror.git",
                authentication: "anonymous",
            }),
            await owner.repository.create({
                accountId,
                requestId: RequestId.create(),
                name: "laptop",
                hosting: "host",
                host: Subject.key(principal.host.reference(ids.account, ids.host)),
            }),
        ];

        // reach the platform repository through the storage: the reader pulls, the owner pushes, strangers see nothing
        const local = { remote: site.remote, credential: null };
        expect([
            await reader.repository.access({ accountId, id: site.id, mode: "pull" }),
            await outcome(reader.repository.access({ accountId, id: site.id, mode: "push" })),
            await owner.repository.access({ accountId, id: site.id, mode: "push" }),
            await outcome(stranger.repository.access({ accountId, id: site.id, mode: "pull" })),
        ]).toEqual([local, "FORBIDDEN", local, "NOT_FOUND"]);

        // pull an anonymous remote unchanged and push to it without a platform credential
        expect([
            await owner.repository.access({ accountId, id: mirror.id, mode: "pull" }),
            await outcome(owner.repository.access({ accountId, id: mirror.id, mode: "push" })),
        ]).toEqual([
            { remote: "https://git.example.test/mirror.git", credential: null },
            "BAD_REQUEST",
        ]);

        // leave a host repository's references and checkouts to its host
        expect([
            await outcome(
                owner.repository.refresh({
                    accountId,
                    id: laptop.id,
                    requestId: RequestId.create(),
                }),
            ),
            await outcome(owner.repository.access({ accountId, id: laptop.id, mode: "pull" })),
        ]).toEqual(["BAD_REQUEST", "BAD_REQUEST"]);
    },
);

test.each(TEST_DIALECTS)(
    "create a committed platform repository's storage through the settlement controller once creating it failed on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const owner = region.connect(ids.owner);
        const accountId = ids.account;

        // commit the repository though its storage fails to create it once
        const create = region.storage.create.bind(region.storage);
        region.storage.create = async () => {
            region.storage.create = create;
            throw new Error("storage unavailable");
        };
        const site = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "site",
            hosting: "platform",
        });
        const kept = await region.database.select().from(settlement);
        expect([
            await readdir(region.storage.directory),
            kept.map((row) => [row.object, row.method, row.target]),
            region.reported,
        ]).toEqual([[], [["repository", "create", site.id]], [new Error("storage unavailable")]]);

        // create it through the host's settlement controller after the grace and the failed settle's claim pass
        await region.database
            .update(settlement)
            .set({ createdAt: kept[0]!.createdAt - 61_000, claimedAt: kept[0]!.createdAt - 61_000 })
            .where(eq(settlement.id, kept[0]!.id));
        const controller = region.server.objects
            .controllers()
            .find((entry) => entry.name === "settlement")!;
        expect(
            await controller.reconcile(kept[0]!.id, reconciliation(AbortSignal.timeout(5000))),
        ).toBeUndefined();
        expect([
            await readdir(region.storage.directory),
            await region.database.select().from(settlement),
        ]).toEqual([[`${site.id}.git`], []]);
    },
);

test.each(TEST_DIALECTS)(
    "remove the storage a creation settles after its repository was purged on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const owner = region.connect(ids.owner);
        const accountId = ids.account;

        // commit the repository though its storage fails to create it once, then purge it
        const create = region.storage.create.bind(region.storage);
        region.storage.create = async () => {
            region.storage.create = create;
            throw new Error("storage unavailable");
        };
        const site = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "site",
            hosting: "platform",
        });
        const [row] = await region.database.select().from(settlement);
        await owner.repository.delete({ accountId, id: site.id, requestId: RequestId.create() });
        await owner.repository.purge({ accountId, id: site.id, requestId: RequestId.create() });

        // settle the creation after its grace and the failed settle's claim pass, and leave no storage
        await region.database
            .update(settlement)
            .set({ createdAt: row!.createdAt - 61_000, claimedAt: row!.createdAt - 61_000 })
            .where(eq(settlement.id, row!.id));
        const controller = region.server.objects
            .controllers()
            .find((entry) => entry.name === "settlement")!;
        await controller.reconcile(row!.id, reconciliation(AbortSignal.timeout(5000)));
        expect([
            await readdir(region.storage.directory),
            await region.database.select().from(settlement),
        ]).toEqual([[], []]);
    },
);

test.each(TEST_DIALECTS)(
    "record the references a host reports of the repository it keeps, and refuse other hosts and its owner on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const owner = region.connect(ids.owner);
        const reader = region.connect(ids.reader);
        const host = region.connectHost(ids.host);
        const accountId = ids.account;
        const laptop = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "laptop",
            hosting: "host",
            host: Subject.key(principal.host.reference(ids.account, ids.host)),
        });

        // report a branch and an annotated tag as the repository's host for readers to see
        const [commit, tag] = ["a".repeat(40), "b".repeat(40)];
        const listing = {
            defaultReference: "refs/heads/main",
            references: [
                { name: "refs/heads/main", object: commit, commit },
                { name: "refs/tags/v1", object: tag, commit },
            ],
        };
        const report = await host.repository.report({
            accountId,
            id: laptop.id,
            requestId: RequestId.create(),
            ...listing,
        });
        expect([
            [report.defaultReference, report.revision],
            (
                await reader.reference.list({
                    accountId,
                    where: Condition.eq("parentId", laptop.id),
                    order: [{ column: "name", direction: "asc" }],
                })
            ).items.map((item) => [item.name, item.object, item.commit, item.deletedAt]),
            (await host.repository.list({ accountId })).items.map((item) => item.name),
        ]).toEqual([
            ["refs/heads/main", 2],
            [
                ["refs/heads/main", commit, commit, null],
                ["refs/tags/v1", tag, commit, null],
            ],
            ["laptop"],
        ]);

        // hide the repository from another host, and forbid its owner reporting for the host
        const refusal = (client: ReturnType<RepositoryFixture["connect"]>) =>
            outcome(
                client.repository.report({
                    accountId,
                    id: laptop.id,
                    requestId: RequestId.create(),
                    ...listing,
                }),
            );
        expect([await refusal(region.connectHost(ids.otherHost)), await refusal(owner)]).toEqual([
            "NOT_FOUND",
            "FORBIDDEN",
        ]);
    },
);
