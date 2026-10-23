import { expect, onTestFinished, test } from "@destack/test";
import { asc, defineTable, Snapshot, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import {
    accessTables,
    Authorizer,
    none,
    Policy,
    principal,
    relation,
    through,
    type TableMapping,
} from "../index.ts";
import { module4, space } from "../test/fixture.ts";
import { AccessFixture } from "../test/access.ts";

/** Profiles living at the universe, which their viewers read. */
const profile = new Policy(module4.package, {
    name: "profile",
    relations: { viewer: { subjects: [principal.user] } },
    permissions: { read: relation("viewer"), share: none() },
    grantedBy: "share",
});

/** Cards in a space, read through the global profile they show by a plain or a typed field. */
const card = new Policy(module4.package, {
    name: "card",
    relations: { profile: { subjects: [profile] }, holder: { subjects: [profile] } },
    permissions: {
        read: through("profile", "read"),
        view: through("holder", "read"),
        share: none(),
    },
    grantedBy: "share",
});

/** The profiles, keyed alone, as global rows are. */
const profileTable = defineTable("example_profile", { id: text("id").primaryKey() });

/** The cards, with their profile as a plain field and as a typed subject. */
const cardTable = defineTable("example_card", {
    id: text("id").primaryKey(),
    scope: text("scope").notNull(),
    profile: text("profile"),
    holderPackage: text("holder_package"),
    holderType: text("holder_type"),
    holderScope: text("holder_scope"),
    holder: text("holder"),
});

/** The mappings of profiles at the universe and of cards in their spaces. */
const mappings: TableMapping[] = [
    { policy: profile, table: profileTable, id: "id", attributes: {}, relations: {} },
    {
        policy: card,
        table: cardTable,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {
            profile: { column: "profile" },
            holder: {
                column: "holder",
                subject: { packageId: "holderPackage", type: "holderType", scope: "holderScope" },
            },
        },
    },
];

test("follow a relation to a global type at the universe, through a plain and a typed field, deciding in memory as in SQL", async () => {
    // store a profile carol views, and a card in a space showing it through both fields
    const storage = await TestDatabase.create([...accessTables, profileTable, cardTable], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const authorizer = new Authorizer([space, profile, card], mappings);
    const access = new AccessFixture(database);
    await access.replicateScope(space.reference("universe", "personal"), space);
    await database.insert(profileTable).values({ id: "p1" });
    await database.insert(cardTable).values({
        id: "c1",
        scope: "personal",
        profile: "p1",
        holderPackage: module4.package.id,
        holderType: "profile",
        holderScope: "universe",
        holder: "p1",
    });
    await access.replicateRelationship(
        profile.reference("universe", "p1"),
        "viewer",
        principal.user.reference("universe", "carol"),
        0,
    );

    // decide each caller's read through either field, in memory and as SQL
    const snapshot = Snapshot.live(database);
    const target = card.reference("personal", "c1");
    const decide = async (caller: string) => {
        const resolved = await authorizer.resolve(snapshot, "personal", {
            grants: [],
            subject: principal.user.reference("universe", caller),
            groups: [],
            now: 1000,
            attributes: {},
            actors: [],
            contacts: [],
            linkSecrets: [],
        });
        const selected = async (name: "read" | "view") =>
            (
                await database
                    .select({ id: cardTable.id })
                    .from(cardTable)
                    .where(authorizer.where(card.permission(name), resolved, cardTable))
                    .orderBy(asc(cardTable.id))
            ).map((row) => row.id);

        return {
            read: (await authorizer.check(snapshot, card.permission("read"), target, resolved))
                .isAllowed,
            view: (await authorizer.check(snapshot, card.permission("view"), target, resolved))
                .isAllowed,
            readable: await selected("read"),
            viewable: await selected("view"),
        };
    };

    // allow the profile's viewer through both fields, and refuse a stranger
    expect({ carol: await decide("carol"), dave: await decide("dave") }).toEqual({
        carol: { read: true, view: true, readable: ["c1"], viewable: ["c1"] },
        dave: { read: false, view: false, readable: [], viewable: [] },
    });
});
