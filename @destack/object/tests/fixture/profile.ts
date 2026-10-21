import { callTables } from "../../src/test/index.ts";
import { schema } from "@destack/schema";
import { principal, relation, universe } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { definePackage } from "@destack/object";
import { defineObject, field } from "../../src/index.ts";
import { space } from "./space.ts";

/** A public profile with an optional handle unique across every space. */
export const profile = defineObject(
    {
        name: "profile",
        plural: "profiles",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            handle: field.string(schema.string().min(1).max(40)).optional(),
        },
        indexes: { handle: { on: ["handle"], across: () => universe } },
        permissions: { read: relation("owner"), manage: relation("owner") },
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage"),
        delete: method.delete("manage"),
    }),
);

/** Profiles, with handles unique across the universe. */
export const profilesPackage = definePackage({ objects: { profile } });

/** The database of one space's profiles. */
export const profilesDatabase = defineDatabase({
    name: "main",
    tables: [...profile.tables, ...callTables],
});
