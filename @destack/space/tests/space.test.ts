import { spaceTables } from "../src/stack/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import {
    accessTables,
    COPY_NAME,
    accessRelationship,
    Authorization,
    Authorizer,
    principal,
} from "@destack/access";
import { copyOwner } from "@destack/access/test";
import { account, user } from "@destack/account/object";
import { type DirectoryDatabase } from "@destack/directory";
import { TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { Feed, type ReplicaRequest, type ReplicaSource, Scope } from "@destack/sync";
import { RequestId } from "@destack/service/request";
import { ServiceError } from "@destack/service/error";
import * as object from "../src/object/index.ts";
import { setting } from "@destack/setting/object";
import { serveObjects } from "../src/server/index.ts";
import {
    cellDatabase,
    copyAccount,
    ids,
    openGlobal,
    openSpace,
    relayAuthorizer,
    serveSpace,
    spaceOptions,
} from "./fixture/space.ts";

test("rename a space to a free address in its account, and refuse a taken or malformed one", async () => {
    // let the owner own the account, and keep a second space of it beside the fixture's
    const database = await openSpace();
    const owner = principal.user.reference("universe", "owner");
    await copyOwner(database, account.reference("universe", ids.account), owner);
    const work = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000009");
    const now = Date.now();
    await database.insert(object.space.table).values({
        id: work,
        scope: ids.account,
        name: "work",
        createdAt: now,
        updatedAt: now,
    });
    await new Authorization(
        new Authorizer([object.space.policy], [object.space.mapping]),
        database,
        () => ({ subjects: [owner], now, attributes: {} }),
    ).create(object.space.reference(ids.account, work), { owner });
    const client = (await serveSpace(database))("owner");
    const rename = (id: string, name: string) =>
        client.space
            .rename({ accountId: ids.account, id, requestId: RequestId.create(), name })
            .then(
                (renamed) => renamed.name,
                (error: { code: string }) => error.code,
            );

    // move the fixture's space to a free address, and refuse it to the other space while taken
    expect(await rename(ids.space, "home")).toBe("home");
    expect(await rename(work, "home")).toBe("CONFLICT");

    // release the address the space moves away from, and refuse malformed addresses
    expect(await rename(ids.space, "personal")).toBe("personal");
    expect(await rename(work, "home")).toBe("home");
    expect([
        await rename(work, "Home"),
        await rename(work, "-home"),
        await rename(work, "main--home"),
    ]).toEqual(["BAD_REQUEST", "BAD_REQUEST", "BAD_REQUEST"]);
});

test("create a space served by the serving region under a zone of its own, and withdraw the zone of a creation whose name is taken", async () => {
    // let the owner own the account, and serve the region's spaces with a key index and directory of their own
    const database = await openSpace();
    const owner = principal.user.reference("universe", "owner");
    await copyOwner(database, account.reference("universe", ids.account), owner);
    const global = await openGlobal();
    const directory = global.directory as DirectoryDatabase;
    const client = (await serveSpace(database, global))("owner");
    const create = (name: string) =>
        client.space.create({ accountId: ids.account, requestId: RequestId.create(), name });

    // create a space served by the region, placed in it and its name claimed in the directory
    const created = await create("home");
    expect([
        await directory.locate(created.id),
        await object.space.lookup(directory, "name", ["home"], ids.account),
    ]).toEqual([
        { id: created.id, scope: ids.account, cell: ids.region, epoch: 1 },
        object.space.reference(ids.account, created.id),
    ]);

    // refuse a second space under the taken name, withdrawing the zone placed for it
    await expect(create("home")).rejects.toEqual(
        new ServiceError("CONFLICT", { defined: true, message: "name is taken" }),
    );
    expect(await directory.list(ids.account)).toEqual([
        { id: created.id, scope: ids.account, cell: ids.region, epoch: 1 },
    ]);
});

test("create the first space of an account its host serves, decided by the copy of the account's access and own row, and conceal the account from a stranger", async () => {
    // let the owner own the account at its home
    const home = await TestDatabase.create("sqlite", spaceTables, { isMigrated: true });
    onTestFinished(() => home.close());
    const scope = account.reference("universe", ids.account);
    await copyAccount(home.database);
    await copyOwner(home.database, scope, principal.user.reference("universe", "owner"));
    const feed = new Feed(home.database, [
        ...accessTables,
        setting.table,
        account.table,
        user.table,
    ]);
    const global = await openGlobal();
    const requests: ReplicaRequest[] = [];
    const replicas: ReplicaSource = {
        stream: (request, signal) => {
            requests.push(request);

            return feed.subscribe(
                relayAuthorizer.replicaOf(request).queries,
                request.after,
                signal,
            );
        },
    };

    // serve the account from a host without spaces
    const host = await TestDatabase.create("sqlite", cellDatabase(), { isMigrated: true });
    onTestFinished(() => host.close());
    const database = host.database;
    const serve = await serveSpace(
        database,
        { ...global, replicas },
        { cell: { hostId: ids.host }, served: [ids.account] },
    );
    const copied = async () => [
        await database.select().from(Scope.table),
        await database.select().from(accessRelationship),
        await database.select().from(account.table),
    ];
    await expect
        .poll(copied)
        .toEqual([
            await home.database.select().from(Scope.table),
            await home.database.select().from(accessRelationship),
            await home.database.select().from(account.table),
        ]);

    // conceal the account from a stranger, and create the owner's first space placed in the host
    const create = (user: string) =>
        serve(user).space.create({
            accountId: ids.account,
            requestId: RequestId.create(),
            name: "notes",
        });
    await expect(create("stranger")).rejects.toEqual(
        new ServiceError("NOT_FOUND", { defined: true, message: `no scope ${ids.account}` }),
    );
    const { id: created } = await create("owner");
    expect(await global.directory.locate(created)).toEqual({
        id: created,
        scope: ids.account,
        cell: ids.host,
        epoch: 1,
    });

    // copy the account's chain up to the universe as its host, leaving out the access rows the host keeps itself, and the global rows the host reads, beside the new space's copies
    const served = serveObjects(await spaceOptions(database));
    const chain = [ids.account, Scope.universe.id].map((scope) => ({
        name: COPY_NAME,
        scope,
        below: ids.account,
        access: true,
        held: [...served.authorizer.held],
        copied: [...served.authorizer.copied],
        rows: [],
    }));
    const own = [served.universeRequest(ids.host)!];
    const byScope = (request: ReplicaRequest) => [request.name, request.scope].join(" ");
    const isOwn = (request: ReplicaRequest) =>
        request.below === ids.account || request.below === ids.host;
    expect(
        requests.filter(isOwn).sort((left, right) => byScope(left).localeCompare(byScope(right))),
    ).toEqual(
        [...chain, ...own].sort((left, right) => byScope(left).localeCompare(byScope(right))),
    );
});
