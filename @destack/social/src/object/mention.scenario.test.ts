import { activity } from "@destack/notification/object";
import { Mention } from "@destack/object/markdown";
import { expect, test } from "@destack/test";
import { mention } from "./mention.ts";
import { article, TestSocial } from "../test/index.ts";

test("record a mention a text gains, notify its subject once, and forget it once the text loses it", async () => {
    await using fixture = await TestSocial.open();
    const { scope, actors } = fixture;
    const draft = await fixture.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        article.call("grant", { ...scope, id: draft.id, relation: "viewer", subject: actors.bob }),
    );
    const edit = (edits: readonly unknown[]) =>
        fixture.call(
            "alice",
            article.call("edit", { ...scope, id: draft.id, field: "body", edits }),
        );
    const state = async () => ({
        mentions: (await fixture.list("alice", mention)).map((row) => [row.field, row.subjectId]),
        notified: (await fixture.list("bob", activity))
            .filter((row) => row.name === "mentioned")
            .map((row) => [row.parentType, row.count]),
    });

    // mention bob at the start of the body
    await edit([{ insert: Mention.node(actors.bob), run: "alice-1", start: 0 }]);
    await fixture.simulation.idle();
    const mentioned = await state();

    // type after the mention, keeping it
    await edit([{ insert: " hi", run: "alice-2", start: 0, after: { run: "alice-1", offset: 0 } }]);
    await fixture.simulation.idle();
    const kept = await state();

    // delete the mention
    await edit([{ delete: [{ run: "alice-1", start: 0, text: 1 }] }]);
    await fixture.simulation.idle();
    const lost = await state();

    // expect one mention notifying bob once, kept through the edit, then gone with the node
    expect({ mentioned, kept, lost }).toEqual({
        mentioned: { mentions: [["body", actors.bob.id]], notified: [["article", 1]] },
        kept: { mentions: [["body", actors.bob.id]], notified: [["article", 1]] },
        lost: { mentions: [], notified: [["article", 1]] },
    });
});
