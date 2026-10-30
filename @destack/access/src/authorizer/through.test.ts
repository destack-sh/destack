import { TestDatabase } from "@destack/db/test";
import { defineTable, Snapshot, sql, text, type Row } from "@destack/db";
import { schema } from "@destack/schema";
import type { ObjectReference, Subject } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import type { Environment } from "@destack/environment";
import { testSimulation } from "@destack/test/world";
import {
    type Access,
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
    union,
} from "../index.ts";
import { TestAccess } from "../test/access.ts";
import { module4 } from "../test/fixture.ts";

/** Realms, root scopes guarded by their wardens. */
const realm = new Policy(module4.package, {
    name: "realm",
    relations: { warden: { subjects: [principal.user] } },
    permissions: { reside: contained(), guard: relation("warden") },
    scope: true,
});

/** Districts in a realm, guarded by their wardens and the realm's, ruled only from their home. */
const district = new Policy(module4.package, {
    name: "district",
    relations: {
        realm: { subjects: [realm], grantedBy: null, isScope: true },
        home: { subjects: [realm], grantedBy: null },
        warden: { subjects: [principal.user] },
    },
    permissions: {
        reside: contained(),
        guard: union(relation("warden"), through("realm", "guard")),
        rule: through("home", "guard"),
    },
    reserved: ["rule"],
    scope: true,
});

/** Vaults in a realm or a district, opened by their keeper or whoever guards their scope. */
const vault = new Policy(module4.package, {
    name: "vault",
    relations: {
        keeper: { subjects: [principal.user], grantedBy: null },
        scope: { subjects: [realm, district], grantedBy: null, isScope: true },
    },
    permissions: { open: union(relation("keeper"), through("scope", "guard")) },
});

/** Tickets in a district, used by whoever may open the vault they relate to, and no role. */
const ticket = new Policy(module4.package, {
    name: "ticket",
    relations: { vault: { subjects: [vault], grantedBy: null } },
    permissions: { use: through("vault", "open") },
    reserved: ["use"],
});

/** Zones, scopes nested in a realm or in each other, guarded by their wardens alone. */
const zone = new Policy(module4.package, {
    name: "zone",
    relations: { warden: { subjects: [principal.user] } },
    permissions: { reside: contained(), guard: relation("warden") },
    scope: true,
});

/** Crates in any scope, opened by whoever guards it and peeked into by whoever lives in it. */
const crate = new Policy(module4.package, {
    name: "crate",
    relations: { scope: { subjects: [], grantedBy: null, isScope: true } },
    permissions: { open: through("scope", "guard"), peek: through("scope", "reside") },
});

/** Passes for a crate, used and peeked through as the crate allows, and by no role. */
const pass = new Policy(module4.package, {
    name: "pass",
    relations: { crate: { subjects: [crate], grantedBy: null } },
    permissions: { use: through("crate", "open"), peek: through("crate", "peek") },
    reserved: ["use", "peek"],
});

/** The realms, in the universe. */
const realmTable = defineTable("example_realm", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
});

/** The districts, each in its realm, with that realm as its home. */
const districtTable = defineTable("example_district", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    home: text("home").notNull(),
});

/** The vaults, with their keeper. */
const vaultTable = defineTable("example_vault", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    keeper: text("keeper").notNull(),
    reach: text("reach").notNull().default("open"),
});

/** The tickets. */
const ticketTable = defineTable("example_ticket", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
});

/** The zones, each in its realm or zone. */
const zoneTable = defineTable("example_zone", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
});

/** The crates. */
const crateTable = defineTable("example_crate", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
});

/** The passes. */
const passTable = defineTable("example_pass", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
});

/** The mappings of every type to its table, open vaults and every crate handed down. */
const mappings: TableMapping[] = [
    { policy: realm, table: realmTable, id: "id", scope: "scope", attributes: {}, relations: {} },
    {
        policy: district,
        table: districtTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: { home: { column: "home", scope: "universe" } },
    },
    {
        policy: vault,
        table: vaultTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: { keeper: { column: "keeper", scope: "universe" } },
        inherited: { reach: "open" },
    },
    { policy: ticket, table: ticketTable, id: "id", scope: "scope", attributes: {}, relations: {} },
    { policy: zone, table: zoneTable, id: "id", scope: "scope", attributes: {}, relations: {} },
    {
        policy: crate,
        table: crateTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {},
        inherited: {},
    },
    { policy: pass, table: passTable, id: "id", scope: "scope", attributes: {}, relations: {} },
];

/** Every policy of the scenario. */
const policies = [realm, district, vault, ticket, zone, crate, pass];

/** The users of the scenario, by name. */
const USERS = [
    "keeper",
    "realm-owner",
    "near-owner",
    "far-owner",
    "realm-warden",
    "far-warden",
    "stranger",
];

/** The realm. */
const crown = realm.reference("universe", "crown");
/** The district with the tickets. */
const near = district.reference("crown", "near");
/** The district beside it. */
const far = district.reference("crown", "far");

/** A user principal by identifier. */
function user(id: string): Subject {
    return principal.user.reference("universe", id);
}

/** Open a database with the realm, its districts and their access rows. */
async function openRealm(environment: Environment) {
    // record the realm and its districts
    const storage = await TestDatabase.create(
        [
            ...accessTables,
            realmTable,
            districtTable,
            vaultTable,
            ticketTable,
            zoneTable,
            crateTable,
            passTable,
        ],
        { environment, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const fixture = new TestAccess(storage.database, environment);
    await fixture.replicateScope(crown, realm);
    await fixture.replicateScope(near, district);
    await fixture.replicateScope(far, district);

    return { storage, fixture };
}

test("decide a related object's permission with the chain of its scope, in memory and in SQL alike", async () => {
    // keep a vault in the realm and one in the far district, and a ticket in the near district for each
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const { storage, fixture } = await openRealm(environment);
    const { database } = storage;
    await database.transaction(async (transaction) => {
        await transaction.insert(realmTable).values({ id: "crown", scope: "universe" });
        await transaction.insert(districtTable).values([
            { id: "near", scope: "crown", home: "crown" },
            { id: "far", scope: "crown", home: "crown" },
        ]);
        await transaction.insert(vaultTable).values([
            { id: "royal", scope: "crown", keeper: "keeper" },
            { id: "sealed", scope: "crown", keeper: "keeper", reach: "closed" },
            { id: "local", scope: "far", keeper: "keeper" },
        ]);
        await transaction.insert(ticketTable).values([
            { id: "upward", scope: "near" },
            { id: "unreached", scope: "near" },
            { id: "sideways", scope: "near" },
        ]);
    });

    // bind an owner on each scope, guard the realm and the far district, and relate each ticket to a vault
    const upward = ticket.reference("near", "upward");
    const unreached = ticket.reference("near", "unreached");
    const sideways = ticket.reference("near", "sideways");
    await fixture.replicateOwner(crown, user("realm-owner"));
    await fixture.replicateOwner(near, user("near-owner"));
    await fixture.replicateOwner(far, user("far-owner"));
    await fixture.replicateRelationship(crown, "warden", user("realm-warden"), 0);
    await fixture.replicateRelationship(far, "warden", user("far-warden"), 0);
    await fixture.replicateRelationship(upward, "vault", vault.reference("crown", "royal"), 0);
    await fixture.replicateRelationship(unreached, "vault", vault.reference("crown", "sealed"), 0);
    await fixture.replicateRelationship(sideways, "vault", vault.reference("far", "local"), 0);

    // decide a permission in memory and in SQL, reporting a disagreement between the two
    const authorizer = new Authorizer(policies, mappings);
    const snapshot = Snapshot.live(database);
    const decide = async (target: ObjectReference, name: string, id: string) => {
        const permission = { ...target, name };
        const isTicket = target.type === ticket.definition.name;
        const table = isTicket ? ticketTable : districtTable;
        const context = AccessContext.of(Caller.of(user(id)), simulation.clock.now());
        const access = await authorizer.resolve(
            snapshot,
            isTicket ? target.scope : target.id,
            context,
        );
        const memory = await authorizer.check(snapshot, permission, target, access);
        const [row] = await database.execute(
            sql`SELECT EXISTS (SELECT 1 FROM ${table} WHERE ${authorizer.where(permission, access, table)} AND ${table.id} = ${target.id}) AS permitted`,
            schema.record(
                schema.string(),
                schema.union([schema.number(), schema.string(), schema.bigint(), schema.boolean()]),
            ),
        );
        const isPermitted = Number(row?.["permitted"]) === 1;

        return [id, memory.isAllowed === isPermitted ? isPermitted : "disagree"] as const;
    };
    const decideAll = async (target: ObjectReference, name: string) =>
        Object.fromEntries(await Promise.all(USERS.map((id) => decide(target, name, id))));

    // use the upward ticket as the realm's vault opens: its keeper, the realm's owner and warden
    expect(await decideAll(upward, "use")).toEqual({
        keeper: true,
        "realm-owner": true,
        "near-owner": false,
        "far-owner": false,
        "realm-warden": true,
        "far-warden": false,
        stranger: false,
    });

    // refuse everyone a ticket for the realm's sealed vault, which the realm hands down to no district
    const refused = {
        keeper: false,
        "realm-owner": false,
        "near-owner": false,
        "far-owner": false,
        "realm-warden": false,
        "far-warden": false,
        stranger: false,
    };
    expect(await decideAll(unreached, "use")).toEqual(refused);

    // refuse everyone the sideways ticket, whose vault is off its chain, as a replica of it would
    expect(await decideAll(sideways, "use")).toEqual({
        keeper: false,
        "realm-owner": false,
        "near-owner": false,
        "far-owner": false,
        "realm-warden": false,
        "far-warden": false,
        stranger: false,
    });

    // rule the near district as its home realm is guarded, refusing the district's owner
    expect(await decideAll(near, "rule")).toEqual({
        keeper: false,
        "realm-owner": true,
        "near-owner": false,
        "far-owner": false,
        "realm-warden": true,
        "far-warden": false,
        stranger: false,
    });
});

test("decide a batch of rows alike whether other scopes are covered or not", async () => {
    // keep a ticket in the near district for a vault in the realm, which the realm's warden guards
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const { storage, fixture } = await openRealm(environment);
    const { database } = storage;
    await database.transaction(async (transaction) => {
        await transaction
            .insert(vaultTable)
            .values({ id: "royal", scope: "crown", keeper: "keeper" });
        await transaction.insert(ticketTable).values({ id: "upward", scope: "near" });
    });
    const upward = ticket.reference("near", "upward");
    await fixture.replicateRelationship(crown, "warden", user("realm-warden"), 0);
    await fixture.replicateRelationship(upward, "vault", vault.reference("crown", "royal"), 0);

    // check the ticket from the realm with its district covered, then with the far district as well
    const authorizer = new Authorizer(policies, mappings);
    const snapshot = Snapshot.live(database);
    const context = AccessContext.of(Caller.of(user("realm-warden")), simulation.clock.now());
    const above = await authorizer.resolve(snapshot, crown.id, context);
    const inside = await authorizer.resolve(snapshot, near.id, context);
    const beside = await authorizer.resolve(snapshot, far.id, context);
    const rows: Row[] = await database.select().from(ticketTable);
    const check = (below: ReadonlyMap<string, Access>) =>
        authorizer.checkRows(snapshot, ticket.permission("use"), above, rows, undefined, below);
    const nearOnly = await check(new Map([["near", inside]]));
    const both = await check(
        new Map([
            ["near", inside],
            ["far", beside],
        ]),
    );

    // permit the ticket alike, as the realm's warden may open its vault
    expect(nearOnly).toEqual(both);
    expect([...nearOnly.permitted]).toEqual([0]);
});

test("decide a scope arrow on the nearest scope only, of any declared type, through an arrow as directly", async () => {
    // keep a crate in an inner zone inside an outer one, and a pass for it in a zone inside the inner one
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const { storage, fixture } = await openRealm(environment);
    const { database } = storage;
    const outer = zone.reference("crown", "outer");
    const inner = zone.reference("outer", "inner");
    const core = zone.reference("inner", "core");
    await fixture.replicateScope(outer, zone);
    await fixture.replicateScope(inner, zone);
    await fixture.replicateScope(core, zone);
    await database.transaction(async (transaction) => {
        await transaction.insert(crateTable).values({ id: "box", scope: "inner" });
        await transaction.insert(passTable).values({ id: "key", scope: "core" });
    });
    const box = crate.reference("inner", "box");
    const key = pass.reference("core", "key");
    await fixture.replicateRelationship(key, "crate", box, 0);

    // guard each scope, and bind the realm's owner
    await fixture.replicateRelationship(crown, "warden", user("realm-warden"), 0);
    await fixture.replicateRelationship(outer, "warden", user("outer-warden"), 0);
    await fixture.replicateRelationship(inner, "warden", user("inner-warden"), 0);
    await fixture.replicateOwner(crown, user("realm-owner"));

    // decide a permission on the crate and the pass in memory and in SQL, for a user or a resident
    const authorizer = new Authorizer(policies, mappings);
    const snapshot = Snapshot.live(database);
    const decide = async (target: ObjectReference, name: string, caller: string) => {
        const permission = { ...target, name };
        const table = target.type === crate.definition.name ? crateTable : passTable;
        const resident = principal.installation.reference(caller.replace("-resident", ""), "app");
        const context = caller.endsWith("-resident")
            ? AccessContext.of(
                  {
                      ...Caller.of(resident),
                      groups: [
                          {
                              ...zone.reference(
                                  caller === "inner-resident" ? "outer" : "crown",
                                  caller.replace("-resident", ""),
                              ),
                              relation: "contained",
                          },
                      ],
                  },
                  simulation.clock.now(),
              )
            : AccessContext.of(Caller.of(user(caller)), simulation.clock.now());
        const access = await authorizer.resolve(snapshot, target.scope, context);
        const memory = await authorizer.check(snapshot, permission, target, access);
        const [row] = await database.execute(
            sql`SELECT EXISTS (SELECT 1 FROM ${table} WHERE ${authorizer.where(permission, access, table)} AND ${table.id} = ${target.id}) AS permitted`,
            schema.record(
                schema.string(),
                schema.union([schema.number(), schema.string(), schema.bigint(), schema.boolean()]),
            ),
        );
        const isPermitted = Number(row?.["permitted"]) === 1;

        return [caller, memory.isAllowed === isPermitted ? isPermitted : "disagree"] as const;
    };
    const decideAll = async (target: ObjectReference, name: string, callers: readonly string[]) =>
        Object.fromEntries(
            await Promise.all(callers.map((caller) => decide(target, name, caller))),
        );
    const wardens = ["inner-warden", "outer-warden", "realm-warden", "realm-owner", "stranger"];
    const residents = ["inner-resident", "outer-resident"];

    // open the crate as its nearest scope's warden or the realm's owner, directly and through the pass
    const opening = {
        "inner-warden": true,
        "outer-warden": false,
        "realm-warden": false,
        "realm-owner": true,
        stranger: false,
    };
    expect(await decideAll(box, "open", wardens)).toEqual(opening);
    expect(await decideAll(key, "use", wardens)).toEqual(opening);

    // peek into the crate as a resident of its nearest scope only, directly and through the pass
    const peeking = { "inner-resident": true, "outer-resident": false };
    expect(await decideAll(box, "peek", residents)).toEqual(peeking);
    expect(await decideAll(key, "peek", residents)).toEqual(peeking);
});
