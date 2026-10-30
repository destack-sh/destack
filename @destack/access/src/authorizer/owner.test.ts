import { TEST_START, testSimulation } from "@destack/test/world";
import { Identifier } from "@destack/schema";
import { defineDatabase, defineTable, Snapshot, asc, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { expect, onTestFinished, test } from "@destack/test";
import { Scope, type ObjectReference, type Subject } from "@destack/sync";
import {
    AccessContext,
    accessRelationship,
    Authorizer,
    Caller,
    Policy,
    principal,
    Relationship,
    type TableMapping,
    through,
} from "../index.ts";
import { TestAccess } from "../test/access.ts";
import {
    account,
    fixtureTables,
    homeMappings,
    module4,
    policies,
    space,
    spaceTable,
} from "../test/fixture.ts";

/** Assets in one scope, each belonging to the space in its holder column. */
const assetTable = defineTable("example_asset", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    holder: text("holder").notNull(),
});

/** Assets belonging to a space in another account's chain, read by the space's readers. */
const asset = new Policy(module4.package, {
    name: "asset",
    relations: { holder: { subjects: [space], grantedBy: null } },
    permissions: { read: through("holder", "read") },
});

/** The assets' mapping, with their holder as owner wherever the holder lives. */
const assetMapping: TableMapping = {
    policy: asset,
    table: assetTable,
    id: "id",
    scope: "scope",
    attributes: {},
    relations: { holder: { column: "holder", scope: "*" } },
    owner: "holder",
};

/** The person reading the assets. */
const carol = principal.user.reference(Scope.universe.id, "carol");

/** The holder's account, enclosing the holder. */
const holding = account.reference(Scope.universe.id, "account-1");

/** The holder space, inside its account. */
const holder = space.reference("account-1", "personal");

/** An account unrelated to the holder. */
const unrelated = account.reference(Scope.universe.id, "account-2");

/** The scope storing the assets, inside a third account. */
const home = space.reference("account-3", "home");

/** Open a database storing assets of a space in another account's home, scopes replicated. */
async function openWithOwner() {
    // migrate the access tables beside the scopes and the assets
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create(
        defineDatabase({ name: "main", tables: [...fixtureTables, assetTable] }),
        { environment: environment, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;

    // replicate the accounts, the holder and the home, and store the holder and an asset of it
    const replicas = new TestAccess(database, environment);
    for (const scope of [holding, unrelated, account.reference(Scope.universe.id, "account-3")]) {
        await replicas.replicateScope(scope, account);
    }
    await replicas.replicateScope(holder, space);
    await replicas.replicateScope(home, space);
    await database.insert(spaceTable).values({ id: holder.id, account: holder.scope });
    await database.insert(assetTable).values({ id: "asset-1", scope: home.id, holder: holder.id });

    // read the assets carol may read, by SQL and by grant trees
    const authorizer = new Authorizer([...policies, asset], [...homeMappings, assetMapping]);
    const readable = async () => {
        const snapshot = Snapshot.live(database);
        const context = AccessContext.of(Caller.of(carol), TEST_START);
        const access = await authorizer.resolve(snapshot, home.id, context);
        const selected = await database
            .select({ id: assetTable.id })
            .from(assetTable)
            .where(authorizer.where(asset.permission("read"), access, assetTable))
            .orderBy(asc(assetTable.id));
        const rows = await database.select().from(assetTable).orderBy(asc(assetTable.id));
        const checked = await authorizer.checkRows(
            snapshot,
            asset.permission("read"),
            access,
            rows,
        );

        return {
            selected: selected.map((row) => row.id),
            checked: rows.flatMap((row, position) =>
                checked.permitted.has(position) ? [row.id] : [],
            ),
        };
    };

    // define a role reading assets in a scope, bound there to carol
    const reader = (scope: ObjectReference) =>
        replicas.replicateRole(
            scope,
            {
                name: "asset-reader",
                description: "Reads assets",
                permissions: [asset.permission("read")],
            },
            carol,
        );

    // bind a role to carol on another object
    const bind = async (object: ObjectReference, role: string, subject: Subject = carol) => {
        await database.transaction(
            async (transaction) =>
                await transaction.insert(accessRelationship).values(
                    Relationship.encode({
                        id: Identifier.create("relationship", environment),
                        object,
                        role,
                        subject,
                        createdAt: 0,
                        expiresAt: null,
                    }),
                ),
        );
    };

    return { readable, reader, bind, replicas };
}

test("cover a row with the roles bound on its owner and on the scopes enclosing the owner, in SQL and in grant trees", async () => {
    const { readable, reader } = await openWithOwner();
    const none = { selected: [], checked: [] };
    const covered = { selected: ["asset-1"], checked: ["asset-1"] };

    // read nothing without a role, and nothing through a role on an unrelated account
    const before = await readable();
    await reader(unrelated);
    const unrelatedRole = await readable();

    // read the asset through a role on its holder, and through one on the holder's account
    await reader(holder);
    const byOwner = await readable();

    expect({ before, unrelatedRole, byOwner }).toEqual({
        before: none,
        unrelatedRole: none,
        byOwner: covered,
    });
});

test("cover a row with a role bound on the account enclosing its owner", async () => {
    const { readable, reader } = await openWithOwner();

    // read the asset through a role defined and bound on the holder's account
    await reader(holding);
    expect(await readable()).toEqual({ selected: ["asset-1"], checked: ["asset-1"] });
});

test("grant nothing through a role bound on an owner when a scope outside the owner's chain defines it", async () => {
    const { readable, reader, bind, replicas } = await openWithOwner();

    // define a reader in an unrelated account, then bind it on the holder
    await replicas.replicateScope(space.reference("account-2", "elsewhere"), space);
    const foreign = await reader(space.reference("account-2", "elsewhere"));
    await bind(holder, foreign);

    // hide the asset, since the role binds only within its scope's chain
    expect(await readable()).toEqual({ selected: [], checked: [] });
});
