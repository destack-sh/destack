import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Snapshot, sql } from "@destack/db";
import { schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { Authorizer, contained, Policy, principal, through, type TableMapping } from "../index.ts";
import {
    accountTable,
    fixtureDatabase,
    groupTable,
    module4,
    spaceTable,
    teamTable,
} from "../test/fixture.ts";
import { AccessFixture } from "../test/access.ts";

/** Realms, root scopes holding districts, read by the principals inside them. */
const realm = new Policy(module4.package, {
    name: "realm",
    permissions: { reside: contained() },
    scope: true,
});

/** Districts, scopes inside a realm, read by the principals inside them and visited by the installations inside them. */
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

/** Parcels, objects living in a realm, read by the installations inside the realm alone. */
const parcel = new Policy(module4.package, {
    name: "parcel",
    permissions: { reside: contained(principal.installation) },
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

test.for(TEST_DIALECTS)(
    "permit a scope's row to the principals inside it and a row living in a scope to the named principal types inside that scope, refusing others, on %s",
    async (dialect) => {
        // place two districts in one realm and one district in another
        const opened = await TestDatabase.create(dialect, fixtureDatabase, { isMigrated: true });
        onTestFinished(() => opened.close());
        const { database } = opened;
        const authorizer = new Authorizer([realm, district, parcel, lot], mappings);
        const scopes = [
            realm.reference("universe", "near"),
            realm.reference("universe", "far"),
            district.reference("near", "inside"),
            district.reference("near", "sibling"),
            district.reference("far", "outside"),
        ];
        for (const scope of scopes) {
            await new AccessFixture(database).copyScope(scope);
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
        await database
            .insert(teamTable)
            .values([{ id: "yard", scope: "inside", member: "nobody" }]);

        // decide a principal living in a scope on a row, resolved in the scope holding the row, in memory and in SQL
        const snapshot = Snapshot.live(database);
        const decide = async (
            target: ObjectReference,
            scope: string,
            subject = principal.installation.reference(scope, "app"),
            name = "reside",
        ) => {
            const caller = { subjects: [subject], now: 1000, attributes: {} };
            const isScope = target.type === realm.name || target.type === district.name;
            const holder = isScope ? target.id : target.scope;
            const access = await authorizer.resolve(snapshot, holder, caller);
            const permission = { ...target, name };
            const memory = await authorizer.check(snapshot, permission, target, access);
            const [row] = await database.execute(
                sql`SELECT CASE WHEN ${authorizer.permits(permission, target, access)} THEN 1 ELSE 0 END AS permitted`,
                schema.record(
                    schema.string(),
                    schema.union([schema.number(), schema.string(), schema.bigint()]),
                ),
            );

            return [scope, memory.isAllowed, Number(row?.["permitted"]) === 1];
        };

        // permit the district to its own installation alone
        const inside = district.reference("near", "inside");
        expect(
            await Promise.all(["inside", "sibling", "near"].map((scope) => decide(inside, scope))),
        ).toEqual([
            ["inside", true, true],
            ["sibling", false, false],
            ["near", false, false],
        ]);

        // permit the realm to the installations of its districts and its own, refusing another realm's
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
    },
);
