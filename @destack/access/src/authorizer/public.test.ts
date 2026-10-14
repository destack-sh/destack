import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
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
import { openFixture } from "../test/database.ts";

/** Spaces anyone may read once shared publicly, as a public home space is. */
const publicSpace = new Policy(module4.package, {
    name: "space",
    relations: { public: { subjects: [anyone.all()] } },
    permissions: { read: relation("public"), update: none(), delete: none(), share: none() },
    grantedBy: "share",
    scope: true,
    partition: true,
});

test.for(TEST_DIALECTS)(
    "admit any signed-in user's read of a public space whose home keeps its own row, on %s",
    async (dialect) => {
        // keep an account and its public space, its own row and its scope row, in the space's home
        const fixture = await openFixture(dialect);
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
        await access.replicateScope(account.reference("universe", "account-1"));
        await access.replicateScope(home, "own");

        // show the space to anyone from the start
        await access.replicateRelationship(
            "personal",
            home,
            "public",
            anyone.reference("*", "*"),
            0,
        );

        // let a stranger who signed in read it in its own scope
        const snapshot = Snapshot.live(database);
        const stranger = await authorizer.resolve(snapshot, "personal", {
            subjects: [principal.user.reference("universe", "erin")],
            now: 1000,
            attributes: {},
        });
        expect(
            await authorizer.check(snapshot, publicSpace.permission("read"), home, stranger),
        ).toEqual({ isAllowed: true });
    },
);
