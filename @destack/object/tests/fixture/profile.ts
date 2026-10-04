import { schema } from "@destack/schema";
import { journal } from "@destack/audit";
import { principal, relation, universe } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { defineService } from "@destack/service";
import { defineObject, field } from "../../src/index.ts";
import { space } from "./space.ts";

/** A public profile with an optional handle unique across every space. */
export const profile = defineObject({
    name: "profile",
    plural: "profiles",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        handle: field.string(schema.string().min(1).max(40)).optional(),
    },
    indexes: { handle: { on: ["handle"], unique: true, across: () => universe } },
    permissions: { read: relation("owner"), manage: relation("owner") },
    methods: (method) => ({
        get: method.get("read"),
        create: method.create("manage"),
        update: method.update("manage"),
        delete: method.delete("manage"),
    }),
});

/** Profiles, with handles the directory keeps unique. */
export const profilesService = defineService("profiles", { objects: { profile } });

/** The database of one space's profiles. */
export const profilesDatabase = defineDatabase({
    name: "main",
    tables: [...profile.tables, journal],
});
