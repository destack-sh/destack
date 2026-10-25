import { type Actor, article, TestSocial } from "../test/index.ts";
import { expect, test } from "@destack/test";
import { favourite, receipt } from "./index.ts";

test("give each reader one receipt and one favourite of an article, marked again in place and private to them", async () => {
    await using kit = await TestSocial.open();
    const { scope, actors } = kit;
    const draft = await kit.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await kit.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "viewer",
            subject: actors.bob,
        }),
    );

    // let the owner and the viewer each mark one receipt and one favourite
    const mark = (actor: Actor) =>
        kit.call(
            actor,
            receipt.call("create", { ...scope, ...TestSocial.host(article, draft.id) }),
        );
    const star = (actor: Actor) =>
        kit.call(
            actor,
            favourite.call("create", { ...scope, ...TestSocial.host(article, draft.id) }),
        );
    await mark("alice");
    await star("alice");
    const theirs = await mark("bob");
    const starred = await star("bob");

    // move the viewer's receipt to the next read, and keep its one star
    const readAt = kit.clock.now() + 1000;
    await kit.simulation.run({ to: readAt });
    const again = await mark("bob");
    const restarred = await star("bob");
    expect([again.id, again.updatedAt, restarred.id]).toEqual([theirs.id, readAt, starred.id]);

    // show each reader only its own
    const owned = async (actor: Actor) => [
        (await kit.list(actor, receipt)).map((row) => row.ownerId),
        (await kit.list(actor, favourite)).map((row) => row.ownerId),
    ];
    expect([await owned("bob"), await owned("alice")]).toEqual([
        [[actors.bob.id], [actors.bob.id]],
        [[actors.alice.id], [actors.alice.id]],
    ]);
});
