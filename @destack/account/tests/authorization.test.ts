import { AccessContext, Caller, type Permission, principal } from "@destack/access";
import { Snapshot } from "@destack/db";
import objectPackage from "@destack/object/package";
import { Identifier } from "@destack/schema";
import { type ObjectReference, RequestId, Scope, type Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { account, group, user } from "../src/object/index.ts";
import accountPackage from "../src/package.ts";
import { TestAccountStore, OWNER_EMAIL } from "../src/test/index.ts";

test("decide access inside an account for its owner and its editors", async () => {
    // create two accounts as their owner, and make the member an editor of the first
    await using accounts = await TestAccountStore.open();
    const owner = await accounts.signIn(OWNER_EMAIL);
    const member = await accounts.signIn("member@example.com");
    const { id: accountId } = await accounts.createAccount(owner);
    const { id: otherAccountId } = await accounts.createAccount(owner);
    const onAccount = account.reference(owner.id, accountId);
    await owner.client(objectPackage).relationship.create({
        scope: onAccount.scope,
        requestId: RequestId.create(accounts.clock.now()),
        object: onAccount,
        relation: "editor",
        subject: member.subject,
    });

    // make a group in each account
    const createGroup = async (scope: Identifier<"account">, name: string) => {
        const created = await owner.client(accountPackage).group.create({
            scope,
            requestId: RequestId.create(accounts.clock.now()),
            name,
        });

        return group.reference(scope, created.id);
    };
    const readers = await createGroup(accountId, "readers");
    const others = await createGroup(otherAccountId, "elsewhere");

    // decide reading and updating a group inside each account as each person
    const { authorizer } = accounts.store;
    const decide = async (subject: Subject, permission: Permission, target: ObjectReference) => {
        const time = accounts.clock.now();
        const credential = {
            kind: "test",
            id: "test-1",
            audience: accountPackage.id,
            verifiedAt: time,
            expiresAt: time + 60_000,
        };
        const snapshot = Snapshot.live(accounts.database);
        const context = AccessContext.of(Caller.of(subject), time, { credential });
        const access = await authorizer.resolve(snapshot, target.scope, context);
        const decision = await authorizer.check(snapshot, permission, target, access);

        return decision.isAllowed ? "allowed" : "denied";
    };
    const decided = await Promise.all(
        [owner.subject, member.subject].map(async (subject) => [
            await decide(subject, group.permission("read"), readers),
            await decide(subject, group.permission("update"), readers),
            await decide(subject, group.permission("read"), others),
            await decide(subject, account.permission("read"), onAccount),
        ]),
    );
    expect(decided).toEqual([
        ["allowed", "allowed", "allowed", "allowed"],
        ["denied", "denied", "denied", "allowed"],
    ]);
});

test("let a person's home follow on their behalf and read their contact from the user row alone, and no other space", async () => {
    // sign a person up, their home on their user row
    await using accounts = await TestAccountStore.open();
    const person = await accounts.signIn("person@example.com");
    const { authorizer } = accounts.store;

    // decide `follow` and `recipient` on the person as their home and as another space
    const decide = async (space: string, permission: "follow" | "recipient") => {
        const subject = principal.space.reference(Scope.universe.id, space);
        const context = AccessContext.of(Caller.of(subject), accounts.store.clock.now());
        const snapshot = Snapshot.live(accounts.database);
        const access = await authorizer.resolve(snapshot, "universe", context);
        const decision = await authorizer.check(
            snapshot,
            user.permission(permission),
            person.subject,
            access,
        );

        return decision.isAllowed;
    };
    const other = Identifier.create("space");
    expect([
        await decide(person.home, "follow"),
        await decide(other, "follow"),
        await decide(person.home, "recipient"),
        await decide(other, "recipient"),
    ]).toEqual([true, false, true, false]);
});
