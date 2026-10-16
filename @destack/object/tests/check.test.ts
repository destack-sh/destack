import { users } from "./fixture/user.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Clock, Identifier, schema } from "@destack/schema";
import { subjectContext, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { Scope, type Subject } from "@destack/sync";
import { defineObject, Intrinsic } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { space, spacePrincipal } from "./fixture/space.ts";

/** The space whose access the checks decide. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000041");

/** A space's relationships, which check access. */
const relationship = defineObject(
    Intrinsic.relationship.definition(space),
    Intrinsic.relationship.methods,
);

/** Alice, who owns the space. */
const alice = principal.user.reference(Scope.universe.id, users.alice);
/** Bob, who reads the space. */
const bob = principal.user.reference(Scope.universe.id, users.bob);
/** Carol, who has no access. */
const carol = principal.user.reference(Scope.universe.id, users.carol);

test("check the caller's own access under its grant, and another subject's for grantors alone", async () => {
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...relationship.tables, space.table, ...callTables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());

    // create a private space owned by alice and read by bob
    const object = space.reference(Scope.universe.id, SPACE_ID);
    const fixture = new AccessFixture(storage.database);
    await fixture.replicateScope(object, "own");
    await fixture.bindOwner(object, alice);
    await fixture.replicateRole(
        object,
        { name: "reader", description: "Reads the space", permissions: [space.permission("read")] },
        bob,
    );
    await storage.database
        .insert(space.table)
        .values({ id: SPACE_ID, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([relationship]),
        policies: [space],
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: relationship.package },
    });

    // check as alice, as bob, and as a token acting for alice under a grant to read the space
    const token = principal.personalAccessToken.reference(users.alice, "token-alice");
    const delegated = {
        actors: [token],
        grant: {
            id: Identifier.create("grant"),
            scope: users.alice,
            permissions: [{ ...space.permission("read"), scope: SPACE_ID }],
        },
    };
    const check = async (
        caller: Subject,
        subject: Subject,
        permission: string,
        isDelegated = false,
    ) => {
        // answer the decision, or the refusal
        const pending = store.query(
            subjectContext(caller, SPACE_ID, isDelegated ? delegated : {}).access(),
            relationship.call("check", { spaceId: SPACE_ID, subject, permission, object }),
        );
        const refused = await refusal(pending);

        return refused === "done" ? await pending : refused;
    };
    const checks = {
        aliceShare: await check(alice, alice, "share"),
        bobRead: await check(bob, bob, "read"),
        bobShare: await check(bob, bob, "share"),
        bobAsksAlice: await check(bob, alice, "read"),
        aliceAsksBob: await check(alice, bob, "read"),
        aliceAsksCarol: await check(alice, carol, "read"),
        tokenRead: await check(alice, alice, "read", true),
        tokenShare: await check(alice, alice, "share", true),
        tokenAsksBob: await check(alice, bob, "read", true),
        unknown: await check(alice, alice, "edit"),
    };

    // answer the caller's own access, limit the token to its grant, and let only grantors ask for others
    const needsShare = ["FORBIDDEN", "checking another subject's access to space needs share"];
    expect(checks).toEqual({
        aliceShare: { isAllowed: true },
        bobRead: { isAllowed: true },
        bobShare: { isAllowed: false },
        bobAsksAlice: needsShare,
        aliceAsksBob: { isAllowed: true },
        aliceAsksCarol: { isAllowed: false },
        tokenRead: { isAllowed: true },
        tokenShare: { isAllowed: false },
        tokenAsksBob: needsShare,
        unknown: ["BAD_REQUEST", "space has no permission edit"],
    });
});
