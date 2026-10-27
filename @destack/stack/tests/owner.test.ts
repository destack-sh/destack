import { AccessContext, Caller } from "@destack/access";
import { space } from "@destack/account/object";
import { Transaction } from "@destack/db";
import { expect, refusal, test } from "@destack/test";
import type { Subject } from "@destack/sync";
import { ids, TestSpace } from "../src/test/index.ts";

test("decide an owner's permissions on a space in its own store through the predefined owner role, for its person and the space itself", async () => {
    // serve a space its person created, with the person and the space itself as owners, and sign a stranger up
    const fixture = await TestSpace.open();
    const served = await fixture.serve();
    const stranger = await fixture.universe.signIn("stranger@example.test");
    const spaceObject = space.reference(ids.account, ids.space);
    const decide = async (subject: Subject, name: "update" | "replicate") => {
        const caller = AccessContext.of(Caller.of(subject), fixture.machine.clock.now());
        const authorization = await served.store.authorize(
            Transaction.reading(fixture.database),
            ids.space,
            caller,
        );

        return refusal(authorization.require(space.permission(name), spaceObject));
    };

    // grant both owners an owner's permission, and refuse them the reserved one and a stranger both
    expect({
        person: await decide(fixture.owner.subject, "update"),
        space: await decide(fixture.host.principal, "update"),
        reserved: await decide(fixture.owner.subject, "replicate"),
        stranger: await decide(stranger.subject, "update"),
    }).toEqual({
        person: "done",
        space: "done",
        reserved: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"],
        stranger: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: update"],
    });
});
