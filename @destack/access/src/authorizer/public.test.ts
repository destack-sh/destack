import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot } from "@destack/db";
import {
    anyone,
    Authorizer,
    none,
    Policy,
    principal,
    relation,
    type TableMapping,
} from "../index.ts";
import {
    account,
    accountTable,
    homeMappings,
    module4,
    policies,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { AccessFixture } from "../test/access.ts";
import { openFixture, sessionCredential } from "../test/database.ts";

/** Spaces anyone may read once shared publicly, as a public home space is. */
const publicSpace = new Policy(module4.package, {
    name: "space",
    relations: { public: { subjects: [anyone.all()] } },
    permissions: { read: relation("public"), update: none(), delete: none(), share: none() },
    grantedBy: "share",
    scope: true,
    partition: true,
});

test("allow any signed-in user's read of a public space whose home stores its row", async () => {
    // store an account and its public space, the space's row and its scope row, in the space's home
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const { database } = fixture;
    const mappings: TableMapping[] = homeMappings.map((mapping) =>
        mapping.policy === space ? { ...mapping, policy: publicSpace } : mapping,
    );
    const authorizer = new Authorizer(
        policies.map((policy) => (policy === space ? publicSpace : policy)),
        mappings,
    );
    const home = publicSpace.reference("account-1", "personal");
    await database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    const access = new AccessFixture(database);
    await access.replicateScope(account.reference("universe", "account-1"), account);
    await access.replicateScope(home, publicSpace);

    // show the space to anyone from the start
    await access.replicateRelationship(home, "public", anyone.reference("*", "*"), 0);

    // let a stranger who signed in read it in their scope
    const snapshot = Snapshot.live(database);
    const stranger = await authorizer.resolve(snapshot, "personal", {
        grants: [],
        subject: principal.user.reference("universe", "erin"),
        actors: [],
        groups: [],
        contacts: [],
        linkSecrets: [],
        credential: sessionCredential("erin", 1000),
        now: 1000,
        attributes: {},
    });
    expect(
        await authorizer.check(snapshot, publicSpace.permission("read"), home, stranger),
    ).toEqual({ isAllowed: true });
});
