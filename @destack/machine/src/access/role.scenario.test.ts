import { accessRelationship } from "@destack/access";
import { type Identifier, nth, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TestSpace } from "../test/index.ts";

/** A person the stack relates first. */
const carol = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000008");

/** A person the stack relates once it changes. */
const dave = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000009");

test("keep a stack's relationship row until it changes, then replace it", async () => {
    const fixture = await TestSpace.open();
    await fixture.serve();

    // declare a reader role bound to one user through the space's stack, as its owner applies it
    const apply = (user: Identifier<"user">) =>
        fixture.submit([], {
            roles: { reader: { description: "Reads installations", permissions: [] } },
            relationships: { readers: { role: "reader", subject: { id: user } } },
        });
    const bound = async () =>
        (await fixture.database.select().from(accessRelationship))
            .filter((row) => row.managerName === "readers")
            .map((row) => [row.id, row.subjectId]);

    // bind it to one user, apply it unchanged, then rebind it to another
    await apply(carol);
    const first = await bound();
    await apply(carol);
    const kept = await bound();
    await apply(dave);
    const replaced = await bound();

    // expect the same row while unchanged and a new one for the other user
    const [id, subject] = nth(replaced, 0);
    expect([kept, replaced.length, subject, id === nth(nth(first, 0), 0)]).toEqual([
        first,
        1,
        dave,
        false,
    ]);
});
