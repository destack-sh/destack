import { expect, refusal, test } from "@destack/test";
import { Scope, Subject, type Subscription } from "@destack/sync";
import { reconciliation } from "@destack/service/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { principal } from "@destack/access";
import { eq } from "@destack/db";
import { zoneTable } from "@destack/directory";
import { RequestId } from "@destack/service/request";
import { readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { QUERIES_SHAPE, settlement } from "@destack/object";
import { aligned, Identifier } from "@destack/schema";
import { PackageId } from "@destack/package";
import {
    reference,
    type ReferenceSelection,
    referenceShape,
    repository,
} from "../src/object/index.ts";
import { History } from "./fixture/git.ts";
import { ids, RepositoryFixture } from "./fixture/repository.ts";

/** Report a call's outcome as its failure code, or as accepted. */
async function outcome(call: Promise<unknown>): Promise<string> {
    const refused = await refusal(call);

    return refused === "done" ? "accepted" : refused[0];
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
        const named = () => claimed(region, accountId, ["site", "mixed", "web"]);
        const site = { site: created.id };
        expect(await named()).toEqual(site);

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
        expect(await named()).toEqual(site);
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

        // push a commit and an annotated tag and observe them and the default branch on refresh
        const origin = remotePath(created.remote);
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
                    where: { parentId: created.id },
                    orderBy: { name: "asc" },
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
        expect(await named()).toEqual({ web: created.id });
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
        const { items } = await reader.reference.list({
            accountId,
            where: { parentId: created.id },
            orderBy: { name: "asc" },
        });
        const main = aligned(items, 0);
        const v1 = aligned(items, 1);
        expect([
            [main.object, main.commit, main.deletedAt, main.revision],
            [v1.object, v1.commit, v1.deletedAt === null, v1.revision],
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
        const [commit] = await history.write(remotePath(created.remote));
        await owner.repository.refresh({
            accountId,
            id: created.id,
            requestId: RequestId.create(),
        });

        // follow the account's references as the reader and as a stranger
        const follow = async (user: string) => {
            const controller = new AbortController();
            const pages = await region.connect(user).replica.stream(
                {
                    name: "objects",
                    shape: QUERIES_SHAPE,
                    scope: accountId,
                    below: accountId,
                    parameters: { queries: { references: { object: "reference" } } },
                },
                { signal: controller.signal },
            );
            const page = await pages[Symbol.asyncIterator]().next();
            controller.abort();
            if (page.done === true) {
                throw new Error("the sync ended before its first page");
            }

            return page.value.changes.map((change) => [change.row["name"], change.row["commit"]]);
        };
        expect(await follow(ids.reader)).toEqual([["refs/heads/main", commit]]);
        expect(await refusal(follow(ids.stranger))).toEqual(["NOT_FOUND", `no scope ${accountId}`]);
    },
);

test.each(TEST_DIALECTS)(
    "stream the references a space selects to the cell serving it as the space inside the account, and none to another space or cell on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const owner = region.connect(ids.owner);
        const accountId = ids.account;

        // observe one branch of a platform repository
        const site = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "site",
            hosting: "platform",
        });
        const history = new History();
        history.commit("main", "index.ts", "export {};\n");
        const [commit] = await history.write(remotePath(site.remote));
        await owner.repository.refresh({ accountId, id: site.id, requestId: RequestId.create() });

        // place a space of the account and one of another account on the host's cell, as the directory's zones
        const [inside, outside] = [Identifier.create("space"), Identifier.create("space")];
        await region.accounts.database.insert(zoneTable).values([
            { id: inside, scope: Scope.universe.id, parent: accountId, cell: ids.host, epoch: 1 },
            { id: outside, scope: Scope.universe.id, parent: ids.other, cell: ids.host, epoch: 1 },
        ]);
        await region.settle();

        // follow the selected branch as a cell for a space below
        const follow = async (host: string, below: string) => {
            const controller = new AbortController();
            const pages = await region.connectHost(host).replica.stream(
                referenceShape.subscription({
                    name: referenceShape.copy(below),
                    scope: accountId,
                    below,
                    parameters: { references: [[site.id, "refs/heads/main"]] },
                }),
                { signal: controller.signal },
            );
            const page = await pages[Symbol.asyncIterator]().next();
            controller.abort();
            if (page.done === true) {
                throw new Error("the sync ended before its first page");
            }

            return page.value.changes.map((change) => [change.row["name"], change.row["commit"]]);
        };

        // copy the branch for the space inside the account, nothing for the other account's space, and refuse a cell not serving the space
        expect([
            await follow(ids.host, inside),
            await follow(ids.host, outside),
            await refusal(follow(ids.otherHost, inside)),
        ]).toEqual([
            [["refs/heads/main", commit]],
            [],
            ["FORBIDDEN", "permission denied: represent"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "let a space inside the account read its private packages through the cell serving it, and no other account's space on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const accountId = ids.account;

        // create a private package as the owner
        const id = PackageId.parse(`package-${Identifier.create("space").slice("space-".length)}`);
        await region.connect(ids.owner).package.create({
            accountId,
            id,
            requestId: RequestId.create(),
            name: "notes",
            visibility: "private",
        });

        // place a space of the account and one of another account on the host's cell, as the directory's zones
        const [inside, outside] = [Identifier.create("space"), Identifier.create("space")];
        await region.accounts.database.insert(zoneTable).values([
            { id: inside, scope: Scope.universe.id, parent: accountId, cell: ids.host, epoch: 1 },
            { id: outside, scope: Scope.universe.id, parent: ids.other, cell: ids.host, epoch: 1 },
        ]);
        await region.settle();

        // read the package as each space through the host's cell
        const read = async (space: string) => {
            const found = await region.connectSpace(ids.host, space).package.get({ accountId, id });

            return found.name;
        };
        expect([await read(inside), await refusal(read(outside))]).toEqual([
            "notes",
            ["NOT_FOUND", `no scope ${accountId}`],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "copy the zones of the accounts in the service's residency, and none of an account in another residency on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);

        // create an account in the us residency as the owner
        const abroad = await region.accounts.user(ids.owner).account.create({
            scope: ids.owner,
            requestId: RequestId.create(),
            handle: "abroad",
            name: "Abroad",
            kind: "shared",
            residencyId: "us",
        });

        // place a space of each account on the host's cell, as the directory's zones
        const [inside, outside] = [Identifier.create("space"), Identifier.create("space")];
        await region.accounts.database.insert(zoneTable).values([
            { id: inside, scope: Scope.universe.id, parent: ids.account, cell: ids.host, epoch: 1 },
            { id: outside, scope: Scope.universe.id, parent: abroad.id, cell: ids.host, epoch: 1 },
        ]);
        await region.settle();

        // copy the eu account's zone alone
        const copied = await region.database.select({ id: zoneTable.id }).from(zoneTable);
        expect(copied.map((row) => row.id)).toEqual([inside]);
    },
);

test.each(TEST_DIALECTS)(
    "reshape a space's copy of its selected references from its position, adding a newly selected branch and keeping the rest on %s",
    async (dialect) => {
        const region = await RepositoryFixture.open(dialect);
        const owner = region.connect(ids.owner);
        const host = region.connectHost(ids.host);
        const accountId = ids.account;

        // observe two branches of a platform repository, and place a space of the account on the host's cell
        const site = await owner.repository.create({
            accountId,
            requestId: RequestId.create(),
            name: "site",
            hosting: "platform",
        });
        const history = new History();
        history.commit("main", "index.ts", "export {};\n");
        history.commit("next", "index.ts", "export const next = true;\n");
        await history.write(remotePath(site.remote));
        await owner.repository.refresh({ accountId, id: site.id, requestId: RequestId.create() });
        const space = Identifier.create("space");
        await region.accounts.database.insert(zoneTable).values({
            id: space,
            scope: Scope.universe.id,
            parent: accountId,
            cell: ids.host,
            epoch: 1,
        });
        await region.settle();

        // read the first page of the space's copy of some branches, from a position and the branches it reflects
        const main: ReferenceSelection["references"][number] = [site.id, "refs/heads/main"];
        const next: ReferenceSelection["references"][number] = [site.id, "refs/heads/next"];
        const first = async (
            references: ReferenceSelection["references"],
            from: Pick<Subscription, "after" | "previous"> = {},
        ) => {
            const controller = new AbortController();
            const pages = await host.replica.stream(
                {
                    ...referenceShape.subscription({
                        name: referenceShape.copy(space),
                        scope: accountId,
                        below: space,
                        parameters: { references },
                    }),
                    ...from,
                },
                { signal: controller.signal },
            );
            const page = await pages[Symbol.asyncIterator]().next();
            controller.abort();
            if (page.done === true) {
                throw new Error("the sync ended before its first page");
            }

            return page.value;
        };

        // follow main and select next as well from main's position
        const snapshot = await first([main]);
        const reshaped = await first([main, next], {
            after: snapshot.position,
            previous: { references: [main] },
        });

        // add next without a snapshot, keeping main
        expect([
            snapshot.changes.map((change) => [change.operation, change.row["name"]]),
            [
                reshaped.reset,
                reshaped.changes.map((change) => [change.operation, change.row["name"]]),
            ],
        ]).toEqual([[["insert", "refs/heads/main"]], [false, [["insert", "refs/heads/next"]]]]);
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
        await history.write(remotePath(site.remote));
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
                    orderBy: { name: "asc" },
                })
            ).items.map((item) => item.name),
            Object.keys(await claimed(region, accountId, ["mirror", "site"])),
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
            await outcome(owner.repository.open({ accountId, id: site.id, mode: "read" })),
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

        // open the platform repository through the storage: the reader pulls, the owner pushes, strangers see nothing
        const local = (mode: "read" | "write") => ({ url: site.remote, mode, headers: {} });
        expect([
            await reader.repository.open({ accountId, id: site.id, mode: "read" }),
            await outcome(reader.repository.open({ accountId, id: site.id, mode: "write" })),
            await owner.repository.open({ accountId, id: site.id, mode: "write" }),
            await outcome(stranger.repository.open({ accountId, id: site.id, mode: "read" })),
        ]).toEqual([local("read"), "FORBIDDEN", local("write"), "NOT_FOUND"]);

        // pull an anonymous remote unchanged and push to it without a platform credential
        expect([
            await owner.repository.open({ accountId, id: mirror.id, mode: "read" }),
            await outcome(owner.repository.open({ accountId, id: mirror.id, mode: "write" })),
        ]).toEqual([
            { url: "https://git.example.test/mirror.git", mode: "read", headers: {} },
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
            await outcome(owner.repository.open({ accountId, id: laptop.id, mode: "read" })),
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
        const first = aligned(kept, 0);
        expect([
            await readdir(region.storage.directory),
            kept.map((row) => [row.object, row.method, row.target]),
            region.reported,
        ]).toEqual([[], [["repository", "create", site.id]], [new Error("storage unavailable")]]);

        // create it through the host's settlement controller after the grace and the failed settle's claim pass
        await region.database
            .update(settlement)
            .set({ createdAt: first.createdAt - 61_000, claimedAt: first.createdAt - 61_000 })
            .where(eq(settlement.id, first.id));
        const controller = settlementController(region);
        expect(
            await controller.reconcile(first.id, reconciliation(AbortSignal.timeout(5000))),
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

        // commit the repository though its storage fails to create it once, and purge it
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
        const row = aligned(await region.database.select().from(settlement), 0);
        await owner.repository.delete({ accountId, id: site.id, requestId: RequestId.create() });
        await owner.repository.purge({ accountId, id: site.id, requestId: RequestId.create() });

        // settle the creation after its grace and the failed settle's claim pass, and leave no storage
        await region.database
            .update(settlement)
            .set({ createdAt: row.createdAt - 61_000, claimedAt: row.createdAt - 61_000 })
            .where(eq(settlement.id, row.id));
        const controller = settlementController(region);
        await controller.reconcile(row.id, reconciliation(AbortSignal.timeout(5000)));
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
                    where: { parentId: laptop.id },
                    orderBy: { name: "asc" },
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
        const hiding = (client: ReturnType<RepositoryFixture["connect"]>) =>
            outcome(
                client.repository.report({
                    accountId,
                    id: laptop.id,
                    requestId: RequestId.create(),
                    ...listing,
                }),
            );
        expect([await hiding(region.connectHost(ids.otherHost)), await hiding(owner)]).toEqual([
            "NOT_FOUND",
            "FORBIDDEN",
        ]);
    },
);

/** Read the local path of a platform repository's file remote. */
function remotePath(remote: string | null): string {
    if (remote === null) {
        throw new TypeError("a platform repository has a remote");
    }

    return fileURLToPath(remote);
}

/** Find the settlement controller a region's object server runs. */
function settlementController(region: RepositoryFixture) {
    const controller = region.forge.objects
        .controllers()
        .find((entry) => entry.name === "settlement");
    if (controller === undefined) {
        throw new TypeError("the object server runs a settlement controller");
    }

    return controller;
}

/** Read which of some names the directory gives a repository of an account, by name. */
async function claimed(
    region: RepositoryFixture,
    accountId: string,
    names: readonly string[],
): Promise<Record<string, string>> {
    const owners = await Promise.all(
        names.map(async (name) => {
            const owner = await repository.lookup(region.directory, "name", [name], accountId);

            return owner === undefined ? [] : [[name, owner.id] as const];
        }),
    );

    return Object.fromEntries(owners.flat());
}
