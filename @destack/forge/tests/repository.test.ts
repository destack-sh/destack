import { expect, refusal } from "@destack/test";
import { and, eq, inArray, isNull, Snapshot } from "@destack/db";
import { secret } from "@destack/vault/object";
import { RequestId } from "@destack/runtime/request";
import { access, readdir } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Claim, ClaimLookup, claimTable, QUERIES_PUBLICATION, settlement } from "@destack/object";
import { nth, Identifier } from "@destack/schema";
import { PackageId } from "@destack/package";
import { reference, referenceShape, repository } from "../src/object/index.ts";
import { History } from "./fixture/git.ts";
import { ids, type RepositoryFixture, repositoryTest } from "./fixture/repository.ts";

/** Report a call's outcome as its failure code, or as accepted. */
async function outcome(call: Promise<unknown>): Promise<string> {
    const refused = await refusal(call);

    return refused === "done" ? "accepted" : refused[0];
}

/** Watch the forge's storage, which the file's tests share, listing what it gains from now on. */
async function storageGains(forge: RepositoryFixture): Promise<() => Promise<string[]>> {
    const before = new Set(await readdir(forge.storage.directory));

    return async () => (await readdir(forge.storage.directory)).filter((name) => !before.has(name));
}

repositoryTest(
    "create a repository in an account its owner created after the forge was installed",
    async ({ forge }) => {
        // create the account and its repository once the forge replicates the account
        const tenant = await forge.tenant();
        const created = await forge.connect(tenant.owner).repository.create({
            spaceId: forge.space,
            accountId: tenant.accountId,
            requestId: RequestId.create(Date.now()),
            name: "site",
            hosting: "platform",
        });
        expect([created.accountId, created.name, created.hosting]).toEqual([
            tenant.accountId,
            "site",
            "platform",
        ]);
    },
);

repositoryTest(
    "create, rename and refresh a platform repository as its account's owner, and refuse readers and strangers",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const owner = forge.connect(tenant.owner);
        const reader = forge.connect(tenant.reader);
        const stranger = forge.connect(tenant.stranger);
        const accountId = tenant.accountId;

        // create a platform repository with its name in the directory and its storage on the platform
        const created = await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(Date.now()),
            name: "site",
            hosting: "platform",
        });
        expect([
            created.scope,
            created.name,
            created.hosting,
            created.machineId,
            created.provider,
            created.providerRepositoryId,
            created.remote,
            created.defaultReference,
            created.authentication,
            created.connectionId,
            created.revision,
        ]).toEqual([
            forge.space,
            "site",
            "platform",
            null,
            "local",
            created.id,
            `file://${forge.storage.directory}/${created.id}.git`,
            null,
            null,
            null,
            1,
        ]);
        const named = () => claimed(forge, accountId, ["site", "mixed", "web"]);
        const site = { site: created.id };
        expect(await named()).toEqual(site);

        // refuse a taken name and an origin mixing platform and remote fields and store neither
        const refused = [
            await outcome(
                owner.repository.create({
                    spaceId: forge.space,
                    accountId,
                    requestId: RequestId.create(Date.now()),
                    name: "site",
                    hosting: "platform",
                }),
            ),
            await outcome(
                owner.repository.create({
                    spaceId: forge.space,
                    accountId,
                    requestId: RequestId.create(Date.now()),
                    name: "mixed",
                    hosting: "platform",
                    remote: "https://example.test/mixed.git",
                }),
            ),
        ];
        expect(refused).toEqual(["CONFLICT", "BAD_REQUEST"]);
        expect(await named()).toEqual(site);
        expect(await stored(forge, [created.id])).toEqual([created.id]);

        // let the reader read, forbid its writes, and show strangers nothing in the public home
        expect([
            (
                await reader.repository.list({
                    spaceId: forge.space,
                    where: { accountId },
                })
            ).items.map((item) => item.name),
            await outcome(reader.repository.get({ spaceId: forge.space, id: created.id })),
            await outcome(
                reader.repository.create({
                    spaceId: forge.space,
                    accountId,
                    requestId: RequestId.create(Date.now()),
                    name: "other",
                    hosting: "platform",
                }),
            ),
            await outcome(
                reader.repository.refresh({
                    spaceId: forge.space,
                    id: created.id,
                    requestId: RequestId.create(Date.now()),
                }),
            ),
            (
                await stranger.repository.list({ spaceId: forge.space, where: { accountId } })
            ).items.map((item) => item.name),
            await outcome(stranger.repository.get({ spaceId: forge.space, id: created.id })),
        ]).toEqual([["site"], "accepted", "FORBIDDEN", "FORBIDDEN", [], "NOT_FOUND"]);

        // push a commit and an annotated tag and observe them and the default branch on refresh
        const origin = remotePath(created.remote);
        const history = new History();
        history.tag("v1", history.commit("main", "README.md", "# Site\n"));
        const [commit, tag] = await history.write(origin);
        const refreshed = await owner.repository.refresh({
            spaceId: forge.space,
            id: created.id,
            requestId: RequestId.create(Date.now()),
        });
        expect([refreshed.defaultReference, refreshed.revision]).toEqual(["refs/heads/main", 2]);
        const references = async () =>
            (
                await reader.reference.list({
                    spaceId: forge.space,
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
            spaceId: forge.space,
            id: created.id,
            requestId: RequestId.create(Date.now()),
            revision: 2,
            name: "web",
        });
        expect([renamed.name, renamed.revision]).toEqual(["web", 3]);
        expect(await named()).toEqual({ web: created.id });
        expect(
            await outcome(
                owner.repository.update({
                    spaceId: forge.space,
                    id: created.id,
                    requestId: RequestId.create(Date.now()),
                    remote: "https://example.test/web.git",
                }),
            ),
        ).toBe("BAD_REQUEST");

        // record a moved branch and a deleted tag on the next refresh and leave unchanged rows
        history.commit("main", "README.md", "# Web\n");
        history.delete("refs/tags/v1");
        const [moved] = await history.write(origin);
        await owner.repository.refresh({
            spaceId: forge.space,
            id: created.id,
            requestId: RequestId.create(Date.now()),
        });
        const { items } = await reader.reference.list({
            spaceId: forge.space,
            where: { parentId: created.id },
            orderBy: { name: "asc" },
        });
        const main = nth(items, 0);
        const v1 = nth(items, 1);
        expect([
            [main.object, main.commit, main.deletedAt, main.revision],
            [v1.object, v1.commit, v1.deletedAt === null, v1.revision],
        ]).toEqual([
            [moved, moved, null, 2],
            [tag, commit, false, 2],
        ]);
    },
);

repositoryTest(
    "sync a repository's references to its readers and none to strangers",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const owner = forge.connect(tenant.owner);
        const accountId = tenant.accountId;

        // observe one branch
        const created = await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(Date.now()),
            name: "notes",
            hosting: "platform",
        });
        const history = new History();
        history.commit("main", "notes.md", "first\n");
        const [commit] = await history.write(remotePath(created.remote));
        await owner.repository.refresh({
            spaceId: forge.space,
            id: created.id,
            requestId: RequestId.create(Date.now()),
        });

        // follow the account's references as the reader and as a stranger
        const follow = async (user: string) => {
            const controller = new AbortController();
            const pages = await forge.connect(user).store.stream(
                {
                    name: "objects",
                    publication: QUERIES_PUBLICATION,
                    scope: forge.space,
                    target: forge.space,
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
        expect(await follow(tenant.reader)).toEqual([["refs/heads/main", commit]]);
        // follow the public home as a stranger, seeing none of the account's references
        expect(await follow(tenant.stranger)).toEqual([]);
    },
);

repositoryTest(
    "stream the references a space inside the account selects to the space, and none to another account's space however it follows",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const other = await forge.tenant();
        const owner = forge.connect(tenant.owner);
        const accountId = tenant.accountId;

        // observe one branch of a platform repository
        const site = await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(Date.now()),
            name: "site",
            hosting: "platform",
        });
        const history = new History();
        history.commit("main", "index.ts", "export {};\n");
        const [commit] = await history.write(remotePath(site.remote));
        await owner.repository.refresh({
            spaceId: forge.space,
            id: site.id,
            requestId: RequestId.create(Date.now()),
        });

        // create a space of the account and one of another account, as their owner does
        const inside = await forge.createSpace(accountId);
        const outside = await forge.createSpace(other.accountId);
        await forge.settle();

        // follow the selected branch as a space for a target space
        const follow = async (space: Identifier<"space">, target: string) => {
            const controller = new AbortController();
            const pages = await forge
                .connectSpace(space)
                .store.stream(
                    referenceShape.subscription(
                        { scope: forge.space, repository: site.id, name: "refs/heads/main" },
                        target,
                    ),
                    { signal: controller.signal },
                );
            const page = await pages[Symbol.asyncIterator]().next();
            controller.abort();
            if (page.done === true) {
                throw new Error("the sync ended before its first page");
            }

            return page.value.changes.map((change) => [change.row["name"], change.row["commit"]]);
        };

        // replicate the branch for the space inside the account, and nothing for the other account's space even where it follows for the inside one
        expect([
            await follow(inside, inside),
            await follow(outside, outside),
            await follow(outside, inside),
        ]).toEqual([[["refs/heads/main", commit]], [], []]);
    },
);

repositoryTest(
    "let a space inside the account read its private packages, and no other account's space",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const other = await forge.tenant();
        const accountId = tenant.accountId;

        // create a private package as the owner
        const id = PackageId.parse(`package-${Identifier.create("space").slice("space-".length)}`);
        await forge.connect(tenant.owner).package.create({
            spaceId: forge.space,
            accountId,
            id,
            requestId: RequestId.create(Date.now()),
            name: "notes",
            visibility: "private",
        });

        // create a space of the account and one of another account, as their owner does
        const inside = await forge.createSpace(accountId);
        const outside = await forge.createSpace(other.accountId);
        await forge.settle();

        // read the package as each space
        const read = async (space: Identifier<"space">) => {
            const found = await forge.connectSpace(space).package.get({ spaceId: forge.space, id });

            return found.name;
        };
        expect([await read(inside), await refusal(read(outside))]).toEqual([
            "notes",
            ["NOT_FOUND", `no package ${id}`],
        ]);
    },
);

repositoryTest(
    "delete and restore a repository through the trash, and purge its storage and name with it",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const owner = forge.connect(tenant.owner);
        const reader = forge.connect(tenant.reader);
        const accountId = tenant.accountId;

        // store a platform repository with one observed branch, and an anonymous Git remote
        const site = await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(Date.now()),
            name: "site",
            hosting: "platform",
        });
        const history = new History();
        history.commit("main", "README.md", "# Site\n");
        await history.write(remotePath(site.remote));
        await owner.repository.refresh({
            spaceId: forge.space,
            id: site.id,
            requestId: RequestId.create(Date.now()),
        });
        const mirror = await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(Date.now()),
            name: "mirror",
            hosting: "git",
            remote: "https://git.example.test/mirror.git",
            authentication: "anonymous",
        });
        const state = async () => [
            (
                await owner.repository.list({
                    spaceId: forge.space,
                    where: { accountId },
                    orderBy: { name: "asc" },
                })
            ).items.map((item) => item.name),
            await held(forge, accountId, ["mirror", "site"]),
            await stored(forge, [site.id, mirror.id]),
            (
                await forge.database
                    .select()
                    .from(reference.table)
                    .where(inArray(reference.table.parentId, [site.id, mirror.id]))
            ).map((row) => row.name),
        ];

        // forbid the reader and move both repositories with their content to the trash
        expect(
            await outcome(
                reader.repository.delete({
                    spaceId: forge.space,
                    id: site.id,
                    requestId: RequestId.create(Date.now()),
                }),
            ),
        ).toBe("FORBIDDEN");
        for (const id of [site.id, mirror.id]) {
            await owner.repository.delete({
                spaceId: forge.space,
                id,
                requestId: RequestId.create(Date.now()),
            });
        }
        expect(await state()).toEqual([[], ["mirror", "site"], [site.id], ["refs/heads/main"]]);

        // refuse checkouts and refreshes of a repository in the trash
        expect([
            await outcome(
                owner.repository.open({ spaceId: forge.space, id: site.id, mode: "read" }),
            ),
            await outcome(
                owner.repository.refresh({
                    spaceId: forge.space,
                    id: site.id,
                    requestId: RequestId.create(Date.now()),
                }),
            ),
        ]).toEqual(["CONFLICT", "CONFLICT"]);

        // restore the platform repository unchanged
        await owner.repository.restore({
            spaceId: forge.space,
            id: site.id,
            requestId: RequestId.create(Date.now()),
        });
        expect(await state()).toEqual([
            ["site"],
            ["mirror", "site"],
            [site.id],
            ["refs/heads/main"],
        ]);

        // purge both, removing the platform repository's storage and references and both names
        await owner.repository.delete({
            spaceId: forge.space,
            id: site.id,
            requestId: RequestId.create(Date.now()),
        });
        for (const id of [site.id, mirror.id]) {
            await owner.repository.purge({
                spaceId: forge.space,
                id,
                requestId: RequestId.create(Date.now()),
            });
        }
        expect(await state()).toEqual([[], [], [], []]);
    },
);

repositoryTest(
    "issue checkout access to pull and push, requiring the push permission and refusing origins the platform has no credential for",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const owner = forge.connect(tenant.owner);
        const reader = forge.connect(tenant.reader);
        const stranger = forge.connect(tenant.stranger);
        const accountId = tenant.accountId;
        const [site, mirror, laptop] = [
            await owner.repository.create({
                spaceId: forge.space,
                accountId,
                requestId: RequestId.create(Date.now()),
                name: "site",
                hosting: "platform",
            }),
            await owner.repository.create({
                spaceId: forge.space,
                accountId,
                requestId: RequestId.create(Date.now()),
                name: "mirror",
                hosting: "git",
                remote: "https://git.example.test/mirror.git",
                authentication: "anonymous",
            }),
            await owner.repository.create({
                spaceId: forge.space,
                accountId,
                requestId: RequestId.create(Date.now()),
                name: "laptop",
                hosting: "machine",
                machineId: ids.machine,
            }),
        ];

        // open the platform repository through the storage: the reader pulls, the owner pushes, strangers see nothing
        const local = (mode: "read" | "write") => ({ url: site.remote, mode, headers: {} });
        expect([
            await reader.repository.open({ spaceId: forge.space, id: site.id, mode: "read" }),
            await outcome(
                reader.repository.open({ spaceId: forge.space, id: site.id, mode: "write" }),
            ),
            await owner.repository.open({ spaceId: forge.space, id: site.id, mode: "write" }),
            await outcome(
                stranger.repository.open({ spaceId: forge.space, id: site.id, mode: "read" }),
            ),
        ]).toEqual([local("read"), "FORBIDDEN", local("write"), "NOT_FOUND"]);

        // pull an anonymous remote unchanged and push to it without a platform credential
        expect([
            await owner.repository.open({ spaceId: forge.space, id: mirror.id, mode: "read" }),
            await outcome(
                owner.repository.open({ spaceId: forge.space, id: mirror.id, mode: "write" }),
            ),
        ]).toEqual([
            { url: "https://git.example.test/mirror.git", mode: "read", headers: {} },
            "BAD_REQUEST",
        ]);

        // leave a machine repository's references and checkouts to its machine
        expect([
            await outcome(
                owner.repository.refresh({
                    spaceId: forge.space,
                    id: laptop.id,
                    requestId: RequestId.create(Date.now()),
                }),
            ),
            await outcome(
                owner.repository.open({ spaceId: forge.space, id: laptop.id, mode: "read" }),
            ),
        ]).toEqual(["BAD_REQUEST", "BAD_REQUEST"]);
    },
);

repositoryTest(
    "refuse a platform repository its storage fails to create, leaving no record, storage or settlement",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const gained = await storageGains(forge);
        const owner = forge.connect(tenant.owner);
        const create = () =>
            owner.repository.create({
                spaceId: forge.space,
                accountId: tenant.accountId,
                requestId: RequestId.create(Date.now()),
                name: "site",
                hosting: "platform",
            });

        // fail the storage's creation once, refusing the repository before its record commits
        const created = forge.storage.create.bind(forge.storage);
        forge.storage.create = async () => {
            forge.storage.create = created;
            throw new Error("storage unavailable");
        };
        const refused = await outcome(create());

        // create the name again once the storage answers
        const site = await create();
        expect([refused, await gained(), await settlements(forge, site.id)]).toEqual([
            "INTERNAL_SERVER_ERROR",
            [`${site.id}.git`],
            [],
        ]);
    },
);

repositoryTest(
    "delete the storage a creation made when its record rolls back on a taken name",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const owner = forge.connect(tenant.owner);
        const create = () =>
            owner.repository.create({
                spaceId: forge.space,
                accountId: tenant.accountId,
                requestId: RequestId.create(Date.now()),
                name: "site",
                hosting: "platform",
            });

        // create the name twice: the second creation makes storage, then its record conflicts
        const gained = await storageGains(forge);
        const site = await create();
        const refused = await outcome(create());
        expect([refused, await gained()]).toEqual(["CONFLICT", [`${site.id}.git`]]);
    },
);

repositoryTest(
    "record the references a machine reports of the repository it hosts, and refuse other machines and its owner",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const owner = forge.connect(tenant.owner);
        const reader = forge.connect(tenant.reader);
        const machine = forge.connectMachine(ids.machine);
        const accountId = tenant.accountId;
        const laptop = await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(Date.now()),
            name: "laptop",
            hosting: "machine",
            machineId: ids.machine,
        });

        // report a branch and an annotated tag as the repository's machine for readers to see
        const [commit, tag] = ["a".repeat(40), "b".repeat(40)];
        const listing = {
            defaultReference: "refs/heads/main",
            references: [
                { name: "refs/heads/main", object: commit, commit },
                { name: "refs/tags/v1", object: tag, commit },
            ],
        };
        const report = await machine.repository.report({
            spaceId: forge.space,
            id: laptop.id,
            requestId: RequestId.create(Date.now()),
            ...listing,
        });
        expect([
            [report.defaultReference, report.revision],
            (
                await reader.reference.list({
                    spaceId: forge.space,
                    where: { parentId: laptop.id },
                    orderBy: { name: "asc" },
                })
            ).items.map((item) => [item.name, item.object, item.commit, item.deletedAt]),
            (
                await machine.repository.list({
                    spaceId: forge.space,
                    where: { accountId },
                })
            ).items.map((item) => item.name),
        ]).toEqual([
            ["refs/heads/main", 2],
            [
                ["refs/heads/main", commit, commit, null],
                ["refs/tags/v1", tag, commit, null],
            ],
            ["laptop"],
        ]);

        // hide the repository from another machine, and forbid its owner reporting for the machine
        const hiding = (client: ReturnType<RepositoryFixture["connect"]>) =>
            outcome(
                client.repository.report({
                    spaceId: forge.space,
                    id: laptop.id,
                    requestId: RequestId.create(Date.now()),
                    ...listing,
                }),
            );
        expect([await hiding(forge.connectMachine(ids.otherMachine)), await hiding(owner)]).toEqual(
            ["NOT_FOUND", "FORBIDDEN"],
        );
    },
);

/** Read the local path of a platform repository's file remote. */
function remotePath(remote: string | null): string {
    if (remote === null) {
        throw new TypeError("a platform repository has a remote");
    }

    return fileURLToPath(remote);
}

/** List which of some names the repositories of an account claim, in the trash too. */
async function held(
    forge: RepositoryFixture,
    accountId: string,
    names: readonly string[],
): Promise<string[]> {
    const keys = names.map((name) => Claim.key(repository, "name", [accountId, name]).key);
    const claims = Snapshot.live(forge.accounts.database).read(claimTable);
    const taken = await Claim.claimed(claims, {
        index: Claim.index(repository, "name"),
        keys,
    });

    return names.filter((_name, index) => taken.has(nth(keys, index)));
}

/** Read which of some names the directory gives a repository of an account, by name. */
async function claimed(
    forge: RepositoryFixture,
    accountId: string,
    names: readonly string[],
): Promise<Record<string, string>> {
    const claims = ClaimLookup.of(Snapshot.live(forge.accounts.database).read(claimTable));
    const owners = await Promise.all(
        names.map(async (name) => {
            const owner = await Claim.lookup(repository, claims, "name", [accountId, name]);

            return owner === undefined ? [] : [[name, owner.id] as const];
        }),
    );

    return Object.fromEntries(owners.flat());
}

/** List which of some repositories store a directory in the forge's storage. */
async function stored(
    forge: RepositoryFixture,
    repositories: readonly string[],
): Promise<string[]> {
    const present = await Promise.all(
        repositories.map((id) =>
            access(join(forge.storage.directory, `${id}.git`)).then(
                () => true,
                () => false,
            ),
        ),
    );

    return repositories.filter((_id, index) => nth(present, index));
}

/** Read the settlements recorded for a repository's calls. */
function settlements(forge: RepositoryFixture, id: string) {
    return forge.database.select().from(settlement).where(eq(settlement.target, id));
}

/** Encode ada's password as a Git remote's Basic authorization. */
function basic(password: string): string {
    return `Basic ${new TextEncoder().encode(`ada:${password}`).toBase64()}`;
}

repositoryTest(
    "store a Git remote's credential in the forge's vault, lease it to pushers alone, rotate it, refuse a missing or stray credential, and remove it once the remote goes anonymous",
    async ({ forge }) => {
        const tenant = await forge.tenant();
        const owner = forge.connect(tenant.owner);
        const reader = forge.connect(tenant.reader);
        const remote = "https://git.example.test/mirror.git";
        const create = (
            name: string,
            authentication: "anonymous" | "credential",
            credential?: { readonly user: string; readonly password: string },
        ) =>
            owner.repository.create({
                spaceId: forge.space,
                accountId: tenant.accountId,
                requestId: RequestId.create(Date.now()),
                name,
                hosting: "git",
                remote,
                authentication,
                ...(credential === undefined ? {} : { credential }),
            });
        const mirror = await create("mirror", "credential", { user: "ada", password: "token-1" });

        // lease the remote to pull and push with the credential as Basic authentication, refusing a reader the credential
        const open = (mode: "read" | "write") =>
            owner.repository.open({ spaceId: forge.space, id: mirror.id, mode });
        const leased = [
            await open("read"),
            await open("write"),
            await refusal(
                reader.repository.open({ spaceId: forge.space, id: mirror.id, mode: "read" }),
            ),
        ];

        // rotate the credential, then move the remote to anonymous, which purges it from the vault
        const update = (input: object) =>
            owner.repository.update({
                spaceId: forge.space,
                requestId: RequestId.create(Date.now()),
                id: mirror.id,
                ...input,
            });
        await update({ credential: { user: "ada", password: "token-2" } });
        const rotated = await open("read");
        await update({ authentication: "anonymous" });
        const live = await forge.database
            .select({ name: secret.table.name })
            .from(secret.table)
            .where(and(eq(secret.table.name, mirror.id), isNull(secret.table.purgedAt)));

        // refuse a credential remote without its credential, and a credential for any other origin
        const refused = [
            await refusal(create("bare", "credential")),
            await refusal(create("stray", "anonymous", { user: "ada", password: "token-3" })),
        ];
        expect({ leased, rotated, live, refused }).toEqual({
            leased: [
                { url: remote, mode: "read", headers: { authorization: basic("token-1") } },
                { url: remote, mode: "write", headers: { authorization: basic("token-1") } },
                ["FORBIDDEN", "permission denied: push"],
            ],
            rotated: { url: remote, mode: "read", headers: { authorization: basic("token-2") } },
            live: [],
            refused: [
                ["BAD_REQUEST", "a git remote authenticating by credential needs its credential"],
                [
                    "BAD_REQUEST",
                    "only a git remote authenticating by credential takes a credential",
                ],
            ],
        });
    },
);
