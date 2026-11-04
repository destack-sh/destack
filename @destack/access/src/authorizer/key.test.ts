import { TestDatabase } from "@destack/db/test";
import { defineTable, Snapshot, sql, text } from "@destack/db";
import { schema } from "@destack/schema";
import { type ObjectReference, Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import {
    AccessContext,
    accessTables,
    Authorizer,
    Caller,
    contained,
    Policy,
    principal,
    relation,
    type TableMapping,
    through,
} from "../index.ts";
import { TestAccess } from "../test/access.ts";
import { module4 } from "../test/fixture.ts";

/** Realms, the scopes the vaults and seals live in. */
const realm = new Policy(module4.package, {
    name: "realm",
    relations: {},
    permissions: { reside: contained() },
    scope: true,
});

/** Vaults, opened by their keeper, and named by their code. */
const vault = new Policy(module4.package, {
    name: "vault",
    relations: { keeper: { subjects: [principal.user], grantedBy: null } },
    permissions: { open: relation("keeper") },
});

/** Seals on a vault, which name it by its code, broken by whoever opens the vault. */
const seal = new Policy(module4.package, {
    name: "seal",
    relations: { vault: { subjects: [vault], grantedBy: null } },
    permissions: { break: through("vault", "open") },
    reserved: ["break"],
});

/** The realms. */
const realmTable = defineTable("example_key_realm", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
});

/** The vaults, each with its keeper and its unique code. */
const vaultTable = defineTable("example_key_vault", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    code: text("code").notNull().unique(),
    keeper: text("keeper").notNull(),
});

/** The seals, each naming its vault by code. */
const sealTable = defineTable("example_key_seal", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    vault: text("vault").notNull(),
});

/** The mappings of the types to their tables, a seal's vault matched on its code. */
const mappings: TableMapping[] = [
    { policy: realm, table: realmTable, id: "id", scope: "scope", attributes: {}, relations: {} },
    {
        policy: vault,
        table: vaultTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: { keeper: { column: "keeper", scope: "universe" } },
    },
    {
        policy: seal,
        table: sealTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: { vault: { column: "vault", key: "code" } },
    },
];

test("decide a relation that names its subject by key, in memory and in SQL alike, for a person and a credential", async () => {
    // keep a vault coded royal in a realm, a seal naming it by its code, and one naming a code no vault has
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create(
        [...accessTables, realmTable, vaultTable, sealTable],
        {
            environment,
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const crown = realm.reference("universe", "crown");
    await new TestAccess(database, environment).replicateScope(crown, realm);
    await database.transaction(async (transaction) => {
        await transaction.insert(realmTable).values({ id: "crown", scope: "universe" });
        await transaction
            .insert(vaultTable)
            .values({ id: "vault-1", scope: "crown", code: "royal", keeper: "keeper" });
        await transaction.insert(sealTable).values([
            { id: "kept", scope: "crown", vault: "royal" },
            { id: "stray", scope: "crown", vault: "missing" },
        ]);
    });

    // grant the keeper's client opening the royal vault alone
    const keeper = principal.user.reference("universe", "keeper");
    const client = principal.client.reference(Scope.universe.id, "client");
    const granted = await new TestAccess(database, environment).replicateGrant(keeper, client, [
        { ...vault.permission("open"), scope: "crown", objectId: "vault-1" },
    ]);
    const credential: AccessContext = {
        ...AccessContext.of(Caller.of(keeper), simulation.clock.now()),
        actors: [client],
        grants: [{ id: granted, scope: "keeper" }],
    };

    // decide breaking a seal in memory and in SQL, reporting a disagreement between the two
    const authorizer = new Authorizer([realm, vault, seal], mappings);
    const snapshot = Snapshot.live(database);
    const person = (id: string) =>
        AccessContext.of(
            Caller.of(principal.user.reference("universe", id)),
            simulation.clock.now(),
        );
    const decide = async (target: ObjectReference, context: AccessContext) => {
        const permission = { ...target, name: "break" };
        const access = await authorizer.resolve(snapshot, target.scope, context);
        const memory = await authorizer.check(snapshot, permission, target, access);
        const [row] = await database.execute(
            sql`SELECT EXISTS (SELECT 1 FROM ${sealTable} WHERE ${authorizer.where(permission, access, sealTable)} AND ${sealTable.id} = ${target.id}) AS permitted`,
            schema.record(
                schema.string(),
                schema.union([schema.number(), schema.string(), schema.bigint(), schema.boolean()]),
            ),
        );
        const isPermitted = Number(row?.["permitted"]) === 1;

        return memory.isAllowed === isPermitted ? isPermitted : "disagree";
    };

    // let the keeper and their client break the seal naming the vault's code, and nobody one naming a missing code
    const kept = seal.reference("crown", "kept");
    const stray = seal.reference("crown", "stray");
    expect({
        keeper: await decide(kept, person("keeper")),
        stranger: await decide(kept, person("stranger")),
        stray: await decide(stray, person("keeper")),
        client: await decide(kept, credential),
        clientStray: await decide(stray, credential),
    }).toEqual({ keeper: true, stranger: false, stray: false, client: true, clientStray: false });
});
