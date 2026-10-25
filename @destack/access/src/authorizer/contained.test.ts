import { testSimulation } from "@destack/test/world";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { defineTable, Snapshot, sql, text } from "@destack/db";
import { nth, schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import {
    AccessContext,
    Authorizer,
    Caller,
    CONTAINED,
    contained,
    Policy,
    principal,
    relation,
    TableMapping,
    through,
} from "../index.ts";
import {
    accountTable,
    fixtureDatabase,
    groupTable,
    module4,
    regionTable,
    spaceTable,
    teamTable,
} from "../test/fixture.ts";
import { AccessFixture } from "../test/access.ts";
import { sessionCredential } from "../test/database.ts";

/** Realms, root scopes with districts, read by the principals inside them. */
const realm = new Policy(module4.package, {
    name: "realm",
    permissions: { reside: contained() },
    scope: true,
});

/** Districts: scopes in a realm, read by principals inside and visited by installations inside. */
const district = new Policy(module4.package, {
    name: "district",
    permissions: { reside: contained(), visit: contained(principal.installation) },
    scope: true,
});

/** Lots, objects living in a district, looked at by whoever visits the district. */
const lot = new Policy(module4.package, {
    name: "lot",
    relations: { district: { subjects: [district], grantedBy: null, isScope: true } },
    permissions: { look: through("district", "visit") },
});

/** Parcels, objects living in a realm, read only by the installations inside the realm. */
const parcel = new Policy(module4.package, {
    name: "parcel",
    permissions: { reside: contained(principal.installation) },
});

/** The homes, global scopes whose rows the universe stores by identifier alone. */
const homeTable = defineTable("example_home", { id: text("id").primaryKey() });

/** Homes living in a realm, keyed in their scope. */
const placeTable = defineTable("example_place", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
});

/** The realms', districts' and parcels' tables. */
const mappings: TableMapping[] = [
    { policy: realm, table: accountTable, id: "id", scope: "scope", attributes: {}, relations: {} },
    {
        policy: district,
        table: spaceTable,
        id: "id",
        scope: "account",
        attributes: {},
        relations: {},
    },
    { policy: parcel, table: groupTable, id: "id", scope: "scope", attributes: {}, relations: {} },
    { policy: lot, table: teamTable, id: "id", scope: "scope", attributes: {}, relations: {} },
];

test("permit a scope's row to the principals inside it and a row living in a scope to the named principal types inside that scope, refusing others", async () => {
    // place two districts in one realm and one district in another
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const opened = await TestDatabase.create(fixtureDatabase, {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => opened.close());
    const { database } = opened;
    const authorizer = new Authorizer([realm, district, parcel, lot], mappings);
    const scopes = [
        [realm.reference("universe", "near"), realm],
        [realm.reference("universe", "far"), realm],
        [district.reference("near", "inside"), district],
        [district.reference("near", "sibling"), district],
        [district.reference("far", "outside"), district],
    ] as const;
    for (const [scope, policy] of scopes) {
        await new AccessFixture(database, environment).replicateScope(scope, policy);
    }
    await database.insert(accountTable).values([
        { id: "near", scope: "universe" },
        { id: "far", scope: "universe" },
    ]);
    await database.insert(spaceTable).values([
        { id: "inside", account: "near" },
        { id: "sibling", account: "near" },
        { id: "outside", account: "far" },
    ]);

    await database.insert(groupTable).values([{ id: "plot", scope: "near" }]);
    await database.insert(teamTable).values([{ id: "yard", scope: "inside", member: "nobody" }]);

    // decide a principal living in a scope on a row, resolved in the scope of the row, in memory and in SQL
    const snapshot = Snapshot.live(database);
    const decide = async (
        target: ObjectReference,
        scope: string,
        subject = principal.installation.reference(scope, "app"),
        name = "reside",
    ) => {
        const caller = AccessContext.of(Caller.of(subject), 1000);
        const isScope = target.type === realm.name || target.type === district.name;
        const holder = isScope ? target.id : target.scope;
        const access = await authorizer.resolve(snapshot, holder, caller);
        const permission = { ...target, name };
        const memory = await authorizer.check(snapshot, permission, target, access);
        const { table, id } = authorizer.requireMapping(target);
        const [row] = await database.execute(
            sql`SELECT EXISTS (SELECT 1 FROM ${table} WHERE ${authorizer.where(permission, access, table)} AND ${TableMapping.column(table, id)} = ${target.id}) AS permitted`,
            schema.record(
                schema.string(),
                schema.union([schema.number(), schema.string(), schema.bigint(), schema.boolean()]),
            ),
        );

        return [scope, memory.isAllowed, Number(row?.["permitted"]) === 1];
    };

    // permit the district only to its installation
    const inside = district.reference("near", "inside");
    expect(
        await Promise.all(["inside", "sibling", "near"].map((scope) => decide(inside, scope))),
    ).toEqual([
        ["inside", true, true],
        ["sibling", false, false],
        ["near", false, false],
    ]);

    // permit the realm to its installation and those of its districts, refusing another realm's
    const near = realm.reference("universe", "near");
    expect(
        await Promise.all(["inside", "near", "outside"].map((scope) => decide(near, scope))),
    ).toEqual([
        ["inside", true, true],
        ["near", true, true],
        ["outside", false, false],
    ]);

    // permit a parcel to the installations inside its realm, refusing a machine living there and another realm's installation
    const plot = parcel.reference("near", "plot");
    expect([
        await decide(plot, "inside"),
        await decide(plot, "near"),
        await decide(plot, "near", principal.machine.reference("near", "machine-a")),
        await decide(plot, "outside"),
    ]).toEqual([
        ["inside", true, true],
        ["near", true, true],
        ["near", false, false],
        ["outside", false, false],
    ]);

    // permit a lot to the installations inside its district through the district's typed containment, refusing a machine living there and a sibling's installation
    const yard = lot.reference("inside", "yard");
    expect([
        await decide(yard, "inside", undefined, "look"),
        await decide(yard, "inside", principal.machine.reference("inside", "pc"), "look"),
        await decide(yard, "sibling", undefined, "look"),
    ]).toEqual([
        ["inside", true, true],
        ["inside", false, false],
        ["sibling", false, false],
    ]);
});

test("resolve a credential bound to a scope within that scope and the scopes inside it, refusing every other scope", async () => {
    // place a district in each of two realms
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const opened = await TestDatabase.create(fixtureDatabase, {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => opened.close());
    const { database } = opened;
    const authorizer = new Authorizer([realm, district, parcel, lot], mappings);
    for (const [scope, policy] of [
        [realm.reference("universe", "near"), realm],
        [realm.reference("universe", "far"), realm],
        [district.reference("near", "inside"), district],
        [district.reference("far", "outside"), district],
    ] as const) {
        await new AccessFixture(database, environment).replicateScope(scope, policy);
    }

    // resolve a caller whose credential is bound to the near realm in each scope
    const subject = principal.user.reference("universe", "ada");
    const credential = { ...sessionCredential("1", 1000), scope: "near" };
    const caller = {
        subject: subject,
        groups: [],
        credential: credential,
        now: 1000,
        attributes: {},
        actors: [],
        contacts: [],
        linkSecrets: [],
        grants: [],
    };
    const snapshot = Snapshot.live(database);
    const resolved = await Promise.all(
        ["near", "inside", "far", "outside", "universe"].map((scope) =>
            refusal(authorizer.resolve(snapshot, scope, caller)),
        ),
    );

    // resolve the realm and its district, refusing the other realm, its district and the universe above
    const refused = [
        "UNAUTHORIZED",
        "CREDENTIAL_CONFINED",
        "the caller's credential only applies to near",
    ];
    expect(resolved).toEqual(["done", "done", refused, refused, refused]);
});

test("contain a space's principals under its global principal reference", async () => {
    // store homes, global scopes in a realm whose rows the universe stores, and a person relating to one home
    const home = new Policy(module4.package, {
        name: "home",
        permissions: { reside: contained() },
        scope: true,
        isGlobal: true,
    });
    const person = new Policy(module4.package, {
        name: "person",
        relations: { home: { subjects: [home] } },
        permissions: { contact: through("home", "reside") },
    });
    const authorizer = new Authorizer(
        [realm, home, person],
        [
            {
                policy: realm,
                table: accountTable,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {},
            },
            { policy: home, table: homeTable, id: "id", attributes: {}, relations: {} },
            { policy: person, table: groupTable, id: "id", attributes: {}, relations: {} },
        ],
    );
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const opened = await TestDatabase.create([...fixtureDatabase.tables, homeTable], {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => opened.close());
    const { database } = opened;
    const access = new AccessFixture(database, environment);
    await access.replicateScope(realm.reference("universe", "near"), realm);
    await access.replicateScope(home.reference("near", "own"), home);
    await access.replicateScope(home.reference("near", "other"), home);
    await database.insert(accountTable).values({ id: "near", scope: "universe" });
    await database.insert(homeTable).values([{ id: "own" }, { id: "other" }]);
    await database.insert(groupTable).values({ id: "alice", scope: "universe" });
    await access.replicateRelationship(
        person.reference("universe", "alice"),
        "home",
        home.reference("universe", "own"),
        0,
    );

    // decide an installation of each home on the person's contact, in memory and in SQL
    const snapshot = Snapshot.live(database);
    const target = person.reference("universe", "alice");
    const decide = async (scope: string) => {
        const caller = AccessContext.of(
            Caller.of(principal.installation.reference(scope, "app")),
            1000,
        );
        const resolved = await authorizer.resolve(snapshot, "universe", caller);
        const permission = { ...target, name: "contact" };
        const memory = await authorizer.check(snapshot, permission, target, resolved);
        const listed = await authorizer.checkRows(snapshot, permission, resolved, [
            { id: "alice", scope: "universe" },
        ]);

        return [scope, memory.isAllowed, listed.permitted.has(0)];
    };

    // permit the person's home's installation, which the relation has by its global reference, and refuse another home's
    expect([await decide("own"), await decide("other")]).toEqual([
        ["own", true, true],
        ["other", false, false],
    ]);
});

test("contain a global scope's principal in its scope under its universe reference", async () => {
    // store homes in a realm, each a global scope whose principal callers present at the universe
    const home = new Policy(module4.package, {
        name: "home",
        permissions: { reside: contained() },
        scope: true,
        isGlobal: true,
    });
    const authorizer = new Authorizer(
        [realm, home],
        [
            {
                policy: realm,
                table: accountTable,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {},
            },
            {
                policy: home,
                table: placeTable,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {},
            },
        ],
    );
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const opened = await TestDatabase.create([...fixtureDatabase.tables, placeTable], {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => opened.close());
    const { database } = opened;
    const access = new AccessFixture(database, environment);
    await access.replicateScope(realm.reference("universe", "near"), realm);
    await access.replicateScope(home.reference("near", "own"), home);
    await access.replicateScope(home.reference("near", "other"), home);
    await database.insert(placeTable).values([
        { id: "own", scope: "near" },
        { id: "other", scope: "near" },
    ]);

    // decide each home's universe reference on the home object, deciding in the realm
    const snapshot = Snapshot.live(database);
    const target = home.reference("near", "own");
    const decide = async (id: string) => {
        const caller = AccessContext.of(Caller.of(home.reference("universe", id)), 1000);
        const resolved = await authorizer.resolve(snapshot, "near", caller);

        return (await authorizer.check(snapshot, { ...target, name: "reside" }, target, resolved))
            .isAllowed;
    };

    // allow the home itself, and refuse another home
    expect([await decide("own"), await decide("other")]).toEqual([true, false]);
});

test("admit the contents of a realm related as a zone's residents, through an arrow from a gate in another scope, refusing another realm's", async () => {
    // relate the near realm's contents to the zone, and the zone to a gate in the far realm
    const zone = new Policy(module4.package, {
        name: "zone",
        relations: { resident: { subjects: ["realm#contained"], grantedBy: null } },
        permissions: { reside: relation("resident") },
    });
    const gate = new Policy(module4.package, {
        name: "gate",
        relations: { zone: { subjects: [zone], grantedBy: null } },
        permissions: { pass: through("zone", "reside") },
    });
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const opened = await TestDatabase.create(fixtureDatabase, { environment, isMigrated: true });
    onTestFinished(() => opened.close());
    const { database } = opened;
    const authorizer = new Authorizer(
        [realm, district, zone, gate],
        [
            nth(mappings, 0),
            nth(mappings, 1),
            { policy: zone, table: regionTable, id: "id", attributes: {}, relations: {} },
            {
                policy: gate,
                table: groupTable,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {},
            },
        ],
    );
    const fixture = new AccessFixture(database, environment);
    for (const [scope, policy] of [
        [realm.reference("universe", "near"), realm],
        [realm.reference("universe", "far"), realm],
        [district.reference("near", "inside"), district],
        [district.reference("far", "outside"), district],
    ] as const) {
        await fixture.replicateScope(scope, policy);
    }
    await database.insert(accountTable).values([
        { id: "near", scope: "universe" },
        { id: "far", scope: "universe" },
    ]);
    await database.insert(spaceTable).values([
        { id: "inside", account: "near" },
        { id: "outside", account: "far" },
    ]);
    await database.insert(regionTable).values([{ id: "north" }]);
    await database.insert(groupTable).values([{ id: "door", scope: "outside" }]);
    const north = zone.reference("universe", "north");
    const resident = { ...realm.reference("universe", "near"), relation: CONTAINED };
    await fixture.replicateRelationship(north, "resident", resident, 0);
    await fixture.replicateRelationship(gate.reference("outside", "door"), "zone", north, 0);

    // decide an installation of each district on the gate inside its realm, in memory and in SQL
    const snapshot = Snapshot.live(database);
    const door = gate.reference("outside", "door");
    const decide = async (scope: string, realmId: string) => {
        const caller = {
            subject: principal.installation.reference(scope, "app"),
            groups: [{ ...realm.reference("universe", realmId), relation: CONTAINED }],
            now: 1000,
            attributes: {},
            actors: [],
            contacts: [],
            linkSecrets: [],
            grants: [],
        };
        const access = await authorizer.resolve(snapshot, door.scope, caller);
        const permission = { ...door, name: "pass" };
        const memory = await authorizer.check(snapshot, permission, door, access);
        const [row] = await database.execute(
            sql`SELECT EXISTS (SELECT 1 FROM ${groupTable} WHERE ${authorizer.where(permission, access, groupTable)} AND ${groupTable.id} = ${door.id}) AS permitted`,
            schema.record(
                schema.string(),
                schema.union([schema.number(), schema.string(), schema.bigint(), schema.boolean()]),
            ),
        );

        return [scope, memory.isAllowed, Number(row?.["permitted"]) === 1];
    };

    // pass the near realm's installation inside it, and refuse the far realm's living beside the gate
    expect([await decide("inside", "near"), await decide("outside", "far")]).toEqual([
        ["inside", true, true],
        ["outside", false, false],
    ]);
});
