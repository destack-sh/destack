import { AccessContext, Caller } from "@destack/access";
import type { ObjectStore } from "@destack/object/server";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { board, ids, memo, TestPartition } from "../src/test/index.ts";

test("accept a token bound to an account at the account's home, writing an account card there, and refuse it at another space of the account", async () => {
    // host the account's home and another space of the account, each serving the cards
    const { home, other, homed, served } = await TestPartition.open();

    // write an account card at the home with an account-bound token, and read the account's cards at the other space with one
    const created = await home
        .person(homed, home.owner, ids.account)
        .client(board)
        .card.create({
            scope: ids.account,
            requestId: RequestId.create(home.environment),
            title: "Plan",
        });
    await home.simulation.idle();
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
            "PARTITION_TOKEN_DENIED",
            "partition is forbidden: the universe token is bound to another partition",
        ],
    });
});

test("write an account's propagated memo at the account's home, and refuse it as a replica at another space of the account", async () => {
    // host the account's home and another space of the account, each serving the memos
    const { home, homed, served } = await TestPartition.open();

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
            "PRECONDITION_FAILED",
            "REPLICA_BLOCKED",
            `cannot change memo rows of '${ids.account}': they are replicas here, so send changes to their home`,
        ],
    });
});
