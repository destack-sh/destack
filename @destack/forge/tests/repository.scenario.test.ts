import { FOLLOWER_REVISION } from "@destack/sync/test";
import forgePackage from "../src/package.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { and, eq, inArray, isNull, Snapshot } from "@destack/db";
import { secret } from "@destack/vault/object";
import { RequestId, Scope } from "@destack/sync";
import { principal } from "@destack/access";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
    Claim,
    ClaimLookup,
    claimTable,
    QUERIES_PUBLICATION,
    settlement,
    Shape,
} from "@destack/object";
import { nth, Identifier } from "@destack/schema";
import { PackageId } from "@destack/package";
import { reference, referenceShape, repository } from "@destack/registry/object";
import { History } from "@destack/git/test";
import type { TestPerson } from "@destack/account/test";
import { TestForge } from "../src/test/index.ts";

/** Report a call's outcome as its failure code, or as accepted. */
async function outcome(call: Promise<unknown>): Promise<string> {
    const refused = await refusal(call);

    return refused === "done" ? "accepted" : refused[0];
}

/** Watch the forge's storage, which the file's tests share, listing what it gains from now on. */
async function storageGains(forge: TestForge): Promise<() => Promise<string[]>> {
    const { directory, environment } = await forge.storage();
    const list = async () =>
        (await environment.files.readDirectory(directory)).map((entry) => entry.name);
    const before = new Set(await list());

    return async () => (await list()).filter((name) => !before.has(name));
}

test("create, rename and refresh a platform repository as its owner", async () => {
    const forge = await TestForge.open();
    const tenant = await forge.tenant();
    const owner = forge.client(tenant.owner);
    const reader = forge.client(tenant.reader);
    const stranger = forge.client(tenant.stranger);
    const accountId = tenant.accountId;

    // create a platform repository with its name in the directory and its storage on the platform
    const created = await owner.repository.create({
        spaceId: forge.space,
        accountId,
        requestId: RequestId.create(forge.environment),
        name: "site",
        hosting: "platform",
    });
    expect([
        created.scope,
        created.accountId,
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
        accountId,
        "site",
        "platform",
        null,
        "local",
        created.id,
        `file://${(await forge.storage()).directory}/${created.id}.git`,
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
                requestId: RequestId.create(forge.environment),
                name: "site",
                hosting: "platform",
            }),
        ),
        await outcome(
            owner.repository.create({
                spaceId: forge.space,
                accountId,
                requestId: RequestId.create(forge.environment),
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
                requestId: RequestId.create(forge.environment),
                name: "other",
                hosting: "platform",
            }),
        ),
        await outcome(
            reader.repository.refresh({
                spaceId: forge.space,
                id: created.id,
                requestId: RequestId.create(forge.environment),
            }),
        ),
        (await stranger.repository.list({ spaceId: forge.space, where: { accountId } })).items.map(
            (item) => item.name,
        ),
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
        requestId: RequestId.create(forge.environment),
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

    // rename the repository, moving its name in the directory and keeping the old one finding it
    const renamed = await owner.repository.update({
        spaceId: forge.space,
        id: created.id,
        requestId: RequestId.create(forge.environment),
        revision: 2,
        name: "web",
    });
    expect([renamed.name, renamed.revision]).toEqual(["web", 3]);
    expect(await named()).toEqual({ site: created.id, web: created.id });
    expect(
        await refusal(
            owner.repository.update({
                spaceId: forge.space,
                id: created.id,
                requestId: RequestId.create(forge.environment),
                remote: "https://example.test/web.git",
            }),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "REPOSITORY_INVALID",
        "repository is invalid: 'platform' repositories cannot move",
    ]);

    // record a moved branch and a deleted tag on the next refresh and leave unchanged rows
    history.commit("main", "README.md", "# Web\n");
    history.delete("refs/tags/v1");
    const [moved] = await history.write(origin);
    await owner.repository.refresh({
        spaceId: forge.space,
        id: created.id,
        requestId: RequestId.create(forge.environment),
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
});

test("sync an account's references and packages to its readers and spaces", async () => {
    const forge = await TestForge.open();
    const tenant = await forge.tenant();
    const other = await forge.tenant();
    const owner = forge.client(tenant.owner);
    const accountId = tenant.accountId;

    // observe one branch of a platform repository
    const site = await owner.repository.create({
        spaceId: forge.space,
        accountId,
        requestId: RequestId.create(forge.environment),
        name: "site",
        hosting: "platform",
    });
    const history = new History();
    history.commit("main", "index.ts", "export {};\n");
    const [commit] = await history.write(remotePath(site.remote));
    await owner.repository.refresh({
        spaceId: forge.space,
        id: site.id,
        requestId: RequestId.create(forge.environment),
    });

    // read the main branch's row
    const { items } = await owner.reference.list({
        spaceId: forge.space,
        where: { parentId: site.id, name: "refs/heads/main" },
        limit: 1,
    });
    const main = nth(items, 0);

    // create a private package as the owner
    const id = PackageId.parse(
        `package-${Identifier.create("space", forge.environment).slice("space-".length)}`,
    );
    await owner.package.create({
        spaceId: forge.space,
        accountId,
        id,
        requestId: RequestId.create(forge.environment),
        name: "notes",
        visibility: "private",
        role: "app",
    });

    // create a space of the account and one of another account, as their owner does
    const inside = await forge.createSpace(accountId, tenant.owner);
    const outside = await forge.createSpace(other.accountId, other.owner);
    await forge.simulation.idle();

    // follow the account's references as the reader and as a stranger
    const query = async (user: TestPerson) => {
        const controller = new AbortController();
        const connection = forge.home.person(user).connect(forgePackage);
        onTestFinished(() => connection.close());
        const pages = await connection.store.stream(
            {
                name: "objects",
                publication: QUERIES_PUBLICATION,
                scope: forge.space,
                target: forge.space,
                parameters: { queries: { references: reference.query() } },
                revision: FOLLOWER_REVISION,
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
    expect(await query(tenant.reader)).toEqual([["refs/heads/main", commit]]);
    // follow the public home as a stranger, seeing none of the account's references
    expect(await query(tenant.stranger)).toEqual([]);

    // follow the selected branch as a space for a target space
    const follow = async (space: Identifier<"space">, target: string) => {
        const controller = new AbortController();
        const connection = forge.home
            .as(principal.space.reference(Scope.universe.id, space))
            .connect(forgePackage);
        onTestFinished(() => connection.close());
        const pages = await connection.store.stream(
            {
                ...Shape.subscription(
                    forge.space,
                    [
                        {
                            shape: referenceShape,
                            keys: [{ id: main.id }],
                        },
                    ],
                    { target },
                ),
                revision: FOLLOWER_REVISION,
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

    // replicate the branch and its repository to the account's space only
    expect([
        await follow(inside, inside),
        await follow(outside, outside),
        await follow(outside, inside),
    ]).toEqual([
        [
            ["refs/heads/main", commit],
            ["site", undefined],
        ],
        [],
        [],
    ]);

    // read the private package as each space
    const read = async (space: Identifier<"space">) => {
        const found = await forge
            .asRegistry(principal.space.reference(Scope.universe.id, space))
            .package.get({ spaceId: forge.space, id });

        return found.name;
    };
    expect([await read(inside), await refusal(read(outside))]).toEqual([
        "notes",
        ["NOT_FOUND", "NOT_FOUND", `package '${id}' not found`],
    ]);
});

test("delete, restore and purge a repository through the trash", async () => {
    const forge = await TestForge.open();
    const tenant = await forge.tenant();
    const owner = forge.client(tenant.owner);
    const reader = forge.client(tenant.reader);
    const accountId = tenant.accountId;

    // store a platform repository with one observed branch, and an anonymous Git remote
    const site = await owner.repository.create({
        spaceId: forge.space,
        accountId,
        requestId: RequestId.create(forge.environment),
        name: "site",
        hosting: "platform",
    });
    const history = new History();
    history.commit("main", "README.md", "# Site\n");
    await history.write(remotePath(site.remote));
    await owner.repository.refresh({
        spaceId: forge.space,
        id: site.id,
        requestId: RequestId.create(forge.environment),
    });
    const mirror = await owner.repository.create({
        spaceId: forge.space,
        accountId,
        requestId: RequestId.create(forge.environment),
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
            await forge.home.database
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
                requestId: RequestId.create(forge.environment),
            }),
        ),
    ).toBe("FORBIDDEN");
    for (const id of [site.id, mirror.id]) {
        await owner.repository.delete({
            spaceId: forge.space,
            id,
            requestId: RequestId.create(forge.environment),
        });
    }
    expect(await state()).toEqual([[], ["mirror", "site"], [site.id], ["refs/heads/main"]]);

    // refuse checkouts and refreshes of a repository in the trash
    expect([
        await outcome(owner.repository.open({ spaceId: forge.space, id: site.id, mode: "read" })),
        await outcome(
            owner.repository.refresh({
                spaceId: forge.space,
                id: site.id,
                requestId: RequestId.create(forge.environment),
            }),
        ),
    ]).toEqual(["PRECONDITION_FAILED", "PRECONDITION_FAILED"]);

    // restore the platform repository unchanged
    await owner.repository.restore({
        spaceId: forge.space,
        id: site.id,
        requestId: RequestId.create(forge.environment),
    });
    expect(await state()).toEqual([["site"], ["mirror", "site"], [site.id], ["refs/heads/main"]]);

    // purge both, removing the platform repository's storage and references and both names
    await owner.repository.delete({
        spaceId: forge.space,
        id: site.id,
        requestId: RequestId.create(forge.environment),
    });
    for (const id of [site.id, mirror.id]) {
        await owner.repository.purge({
            spaceId: forge.space,
            id,
            requestId: RequestId.create(forge.environment),
        });
    }
    expect(await state()).toEqual([[], [], [], []]);
});

test("lease checkouts of platform and anonymous repositories by permission", async () => {
    const forge = await TestForge.open();
    const tenant = await forge.tenant();
    const owner = forge.client(tenant.owner);
    const reader = forge.client(tenant.reader);
    const stranger = forge.client(tenant.stranger);
    const accountId = tenant.accountId;
    const [site, mirror] = [
        await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(forge.environment),
            name: "site",
            hosting: "platform",
        }),
        await owner.repository.create({
            spaceId: forge.space,
            accountId,
            requestId: RequestId.create(forge.environment),
            name: "mirror",
            hosting: "git",
            remote: "https://git.example.test/mirror.git",
            authentication: "anonymous",
        }),
    ];

    // open the platform repository through the storage: the reader pulls, the owner pushes, strangers see nothing
    const local = (mode: "read" | "write") => ({ url: site.remote, mode, headers: {} });
    expect([
        await reader.repository.open({ spaceId: forge.space, id: site.id, mode: "read" }),
        await refusal(reader.repository.open({ spaceId: forge.space, id: site.id, mode: "write" })),
        await owner.repository.open({ spaceId: forge.space, id: site.id, mode: "write" }),
        await refusal(
            stranger.repository.open({ spaceId: forge.space, id: site.id, mode: "read" }),
        ),
    ]).toEqual([
        local("read"),
        ["FORBIDDEN", "PERMISSION_DENIED", "permission 'push' is forbidden: it is not granted"],
        local("write"),
        ["NOT_FOUND", "NOT_FOUND", `repository '${site.id}' not found`],
    ]);

    // pull an anonymous remote unchanged and push to it without a platform credential
    expect([
        await owner.repository.open({ spaceId: forge.space, id: mirror.id, mode: "read" }),
        await refusal(
            owner.repository.open({ spaceId: forge.space, id: mirror.id, mode: "write" }),
        ),
    ]).toEqual([
        { url: "https://git.example.test/mirror.git", mode: "read", headers: {} },
        [
            "BAD_REQUEST",
            "PUSH_INVALID",
            "push is invalid: anonymous remotes do not take push credentials",
        ],
    ]);
});

test("leave no storage of a platform repository its creation fails", async () => {
    const forge = await TestForge.open();
    const tenant = await forge.tenant();
    const gained = await storageGains(forge);
    const owner = forge.client(tenant.owner);
    const create = () =>
        owner.repository.create({
            spaceId: forge.space,
            accountId: tenant.accountId,
            requestId: RequestId.create(forge.environment),
            name: "site",
            hosting: "platform",
        });

    // take the storage's directory away, refusing the repository before its record commits
    const { directory, environment } = await forge.storage();
    await environment.files.remove(directory, { recursive: true });
    await environment.files.writeTextFile(directory, "");
    const refused = await refusal(create());

    // put the directory back, as the storage answers again
    await environment.files.remove(directory);
    await environment.files.mkdir(directory, { recursive: true });

    // create the name again, then once more: the last creation makes storage, then its record conflicts
    const site = await create();
    const conflict = await outcome(create());
    expect([refused, conflict, await gained(), await settlements(forge, site.id)]).toEqual([
        ["SERVICE_UNAVAILABLE", "STORAGE_UNAVAILABLE", "repository storage is unavailable"],
        "CONFLICT",
        [`${site.id}.git`],
        [],
    ]);
});

test("leave a machine repository's references and checkouts to its machine", async () => {
    const forge = await TestForge.open();
    const tenant = await forge.tenant();
    const owner = forge.client(tenant.owner);
    const reader = forge.client(tenant.reader);
    const accountId = tenant.accountId;
    const machineId = await forge.machine(accountId, tenant.owner);
    const machine = forge.asRegistry(principal.machine.reference(Scope.universe.id, machineId));
    const laptop = await owner.repository.create({
        spaceId: forge.space,
        accountId,
        requestId: RequestId.create(forge.environment),
        name: "laptop",
        hosting: "machine",
        machineId,
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
        requestId: RequestId.create(forge.environment),
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

    // refuse the owner refreshing or opening it
    expect([
        await refusal(
            owner.repository.refresh({
                spaceId: forge.space,
                id: laptop.id,
                requestId: RequestId.create(forge.environment),
            }),
        ),
        await refusal(owner.repository.open({ spaceId: forge.space, id: laptop.id, mode: "read" })),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "MACHINE_REPOSITORY_INVALID",
            "repository is invalid: machine repositories are refreshed and opened through their machine",
        ],
        [
            "BAD_REQUEST",
            "MACHINE_REPOSITORY_INVALID",
            "repository is invalid: machine repositories are refreshed and opened through their machine",
        ],
    ]);

    // hide the repository from another machine, and forbid its owner reporting for the machine
    const hiding = (client: ReturnType<TestForge["asRegistry"]>) =>
        outcome(
            client.repository.report({
                spaceId: forge.space,
                id: laptop.id,
                requestId: RequestId.create(forge.environment),
                ...listing,
            }),
        );
    const other = await forge.tenant();
    const otherMachine = principal.machine.reference(
        Scope.universe.id,
        await forge.machine(other.accountId, other.owner),
    );
    expect([await hiding(forge.asRegistry(otherMachine)), await hiding(owner)]).toEqual([
        "NOT_FOUND",
        "FORBIDDEN",
    ]);
});

/** Read the local path of a platform repository's file remote. */
function remotePath(remote: string | null): string {
    if (remote === null) {
        throw new TypeError("a platform repository has a remote");
    }

    return fileURLToPath(remote);
}

/** List which of some names the repositories of an account claim, in the trash too. */
async function held(
    forge: TestForge,
    accountId: string,
    names: readonly string[],
): Promise<string[]> {
    const keys = names.map((name) => Claim.key(repository, "name", [accountId, name]).key);
    const claims = Snapshot.live(forge.home.universe.storage.database).read(claimTable);
    const taken = await Claim.claimed(claims, {
        index: Claim.index(repository, "name"),
        keys,
    });

    return names.filter((_name, index) => taken.has(nth(keys, index)));
}

/** Read which of some names the directory gives a repository of an account, by name. */
async function claimed(
    forge: TestForge,
    accountId: string,
    names: readonly string[],
): Promise<Record<string, string>> {
    const claims = ClaimLookup.of(
        Snapshot.live(forge.home.universe.storage.database).read(claimTable),
    );
    const owners = await Promise.all(
        names.map(async (name) => {
            const owner = await Claim.lookup(repository, claims, "name", [accountId, name]);

            return owner === undefined ? [] : [[name, owner.id] as const];
        }),
    );

    return Object.fromEntries(owners.flat());
}

/** List which of some repositories store a directory in the forge's storage. */
async function stored(forge: TestForge, repositories: readonly string[]): Promise<string[]> {
    const storage = await forge.storage();
    const present = await Promise.all(
        repositories.map((id) =>
            storage.environment.files.exists(join(storage.directory, `${id}.git`)),
        ),
    );

    return repositories.filter((_id, index) => nth(present, index));
}

/** Read the settlements recorded for a repository's calls. */
function settlements(forge: TestForge, id: string) {
    return forge.home.database.select().from(settlement).where(eq(settlement.target, id));
}

/** Encode ada's password as a Git remote's Basic authorization. */
function basic(password: string): string {
    return `Basic ${new TextEncoder().encode(`ada:${password}`).toBase64()}`;
}

test("keep a Git remote's credential in the forge's vault", async () => {
    const forge = await TestForge.open();
    const tenant = await forge.tenant();
    const owner = forge.client(tenant.owner);
    const reader = forge.client(tenant.reader);
    const remote = "https://git.example.test/mirror.git";
    const create = (
        name: string,
        authentication: "anonymous" | "credential",
        credential?: { readonly user: string; readonly password: string },
    ) =>
        owner.repository.create({
            spaceId: forge.space,
            accountId: tenant.accountId,
            requestId: RequestId.create(forge.environment),
            name,
            hosting: "git",
            remote,
            authentication,
            ...(credential === undefined ? {} : { credential }),
        });
    const mirror = await create("mirror", "credential", { user: "ada", password: "token-1" });
    const credentials = () =>
        forge.home.database
            .select({ name: secret.table.name, currentVersion: secret.table.currentVersion })
            .from(secret.table)
            .where(and(eq(secret.table.name, mirror.id), isNull(secret.table.purgedAt)));

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
    const created = await credentials();
    const update = (input: object) =>
        owner.repository.update({
            spaceId: forge.space,
            requestId: RequestId.create(forge.environment),
            id: mirror.id,
            ...input,
        });
    await update({ credential: { user: "ada", password: "token-2" } });
    const rotated = await open("read");
    const current = await credentials();
    await update({ authentication: "anonymous" });
    const live = await credentials();

    // refuse a credential remote without its credential, and a credential for any other origin
    const refused = [
        await refusal(create("bare", "credential")),
        await refusal(create("stray", "anonymous", { user: "ada", password: "token-3" })),
    ];
    expect({ created, leased, current, rotated, live, refused }).toEqual({
        created: [{ name: mirror.id, currentVersion: 1 }],
        leased: [
            { url: remote, mode: "read", headers: { authorization: basic("token-1") } },
            { url: remote, mode: "write", headers: { authorization: basic("token-1") } },
            ["FORBIDDEN", "PERMISSION_DENIED", "permission 'push' is forbidden: it is not granted"],
        ],
        current: [{ name: mirror.id, currentVersion: 2 }],
        rotated: { url: remote, mode: "read", headers: { authorization: basic("token-2") } },
        live: [],
        refused: [
            [
                "PRECONDITION_FAILED",
                "CREDENTIAL_BLOCKED",
                "a git remote authenticating by credential needs its credential",
            ],
            [
                "PRECONDITION_FAILED",
                "CREDENTIAL_BLOCKED",
                "only a git remote authenticating by credential takes a credential",
            ],
        ],
    });
});
