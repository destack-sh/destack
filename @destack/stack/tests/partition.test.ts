import { AccessContext, Caller } from "@destack/access";
import type { ObjectStore } from "@destack/object/server";
import { serversOf } from "@destack/object/test";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { board, card, ids, memo, PartitionFixture } from "../src/test/index.ts";

test("accept a token bound to an account at the account's home, writing an account card there, and refuse it at another space of the account", async () => {
    // host the account's home and another space of the account, each serving the cards
    const { home, other } = await PartitionFixture.open();
    const homed = await home.serve({ servers: serversOf([card]) });
    const served = await other.serve({ servers: serversOf([card]) });

    // write an account card at the home with an account-bound token, and read the account's cards at the other space with one
    const created = await home
        .person(homed, home.owner, ids.account)
        .client(board)
        .card.create({
            scope: ids.account,
            requestId: RequestId.create(home.environment),
            title: "Plan",
        });
    await home.settle(homed);
    const refused = await refusal(
        other
            .person(served, home.owner, ids.account)
            .client(board)
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

test("write an account's handed-down memo at the account's home, and refuse it as a replica at another space of the account", async () => {
    // host the account's home and another space of the account, each serving the memos
    const { home, other } = await PartitionFixture.open();
    const homed = await home.serve({ servers: serversOf([memo]) });
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
