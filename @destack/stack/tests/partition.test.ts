import { AccessContext, Caller } from "@destack/access";
import { Account, account, space } from "@destack/account/object";
import { defineObject, field, PackageHandle } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { Package } from "@destack/package";
import { serversOf } from "@destack/object/test";
import { Identifier, present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { ids, TestSpace } from "../src/test/index.ts";

/** The package of the cards and memos the spaces serve in addition to the stack. */
const BOARD = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000006",
    name: "@example/board",
    version: "2026.10.0",
});

/** Cards of a space or of an account, written by whoever writes their scope. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: [space, account],
        fields: { title: field.string() },
        permissions: ["read", "write"],
    },
    undefined,
    { package: BOARD },
);

/** The package of the cards. */
const cards = PackageHandle.of(card.package, { objects: { card } });

test("accept a token bound to an account at the account's home, writing an account card there, and refuse it at another space of the account", async () => {
    // host the account's home and another space of the account, each serving the cards
    const options = { packages: [cards] };
    const home = await TestSpace.open({ ...options, id: Account.home(ids.account) });
    const homed = await home.serve({ servers: serversOf([card]) });
    const other = await TestSpace.open({
        ...options,
        world: home,
        owner: home.owner,
        account: ids.account,
        id: Identifier.create("space", home.environment),
        name: "other",
    });
    const served = await other.serve({ servers: serversOf([card]) });

    // write an account card at the home with an account-bound token, and read the account's cards at the other space with one
    const created = await home
        .person(homed, home.owner, ids.account)
        .client(cards)
        .card.create({
            scope: ids.account,
            requestId: RequestId.create(home.environment),
            title: "Plan",
        });
    await home.settle(homed);
    const refused = await refusal(
        other
            .person(served, home.owner, ids.account)
            .client(cards)
            .card.list({ scope: ids.account }),
    );
    expect({ scope: created.scope, title: created.title, refused }).toEqual({
        scope: ids.account,
        title: "Plan",
        refused: [
            "FORBIDDEN",
            "FOREIGN_PARTITION_TOKEN",
            "the universe token is bound to another partition",
        ],
    });
});

/** Notes an account hands down to its spaces, written at the account's home. */
const memo = defineObject(
    {
        name: "memo",
        plural: "memos",
        scope: [space, account],
        inherited: {},
        fields: { title: field.string() },
        permissions: ["read", "write"],
    },
    undefined,
    { package: BOARD },
);

/** The package of the memos. */
const memos = PackageHandle.of(memo.package, { objects: { memo } });

test("write an account's handed-down memo at the account's home, and refuse it as a replica at another space of the account", async () => {
    // host the account's home and another space of the account, each serving the memos
    const options = { packages: [memos] };
    const home = await TestSpace.open({ ...options, id: Account.home(ids.account) });
    const homed = await home.serve({ servers: serversOf([memo]) });
    const other = await TestSpace.open({
        ...options,
        world: home,
        owner: home.owner,
        account: ids.account,
        id: Identifier.create("space", home.environment),
        name: "other",
    });
    const served = await other.serve({ servers: serversOf([memo]) });

    // write an account memo at the home as its owner, and the same at the other space's store
    const write = (store: ObjectStore) =>
        store.mutate(AccessContext.of(Caller.of(home.owner.subject), store.clock.now()), {
            id: RequestId.create(store.environment),
            calls: [memo.call("create", { ...memo.in(ids.account), title: "Plan" })],
        });
    const [created] = await write(homed.store);
    const refused = await refusal(write(served.store));
    expect({ created: present(created, "the memo").scope, refused }).toEqual({
        created: ids.account,
        refused: [
            "CONFLICT",
            "REPLICA_WRITE",
            `memo rows of ${ids.account} are replicas here: send changes to their home`,
        ],
    });
});
