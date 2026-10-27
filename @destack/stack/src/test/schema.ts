import { principal, relation } from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import * as object from "../object/index.ts";
import { Package } from "@destack/package";

/** The notes package the space serves in process in addition to the stack. */
const NOTES = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000004",
    name: "@example/notes",
    version: "2026.9.0",
});

/** A type of the notes package, declared by stacks and waiting for their main database. */
export const gauge = defineObject(
    { name: "gauge", plural: "gauges", scope: space, fields: {}, permissions: ["write"] },
    (method) => ({
        create: method.create("write"),
        get: method.get("write"),
        delete: method.delete("write"),
    }),
    { package: NOTES },
).implement({
    declaration: {
        after: [object.database],
        resolve: async (_name, declared, plan) => {
            // wait while the main database is not applied
            const [main] = await plan.transaction.read(object.database.table).findMany({
                where: {
                    scope: schema.identifier("space").parse(plan.stack.scope),
                    name: "main",
                },
            });
            if (main?.conditions["Ready"]?.reason !== "Applied") {
                plan.wait("main is not ready");
            }

            return declared;
        },
    },
});

/** A type the installed notes package declares, which stacks declare once the store serves it. */
export const meter = defineObject(
    { name: "meter", plural: "meters", scope: space, fields: {}, permissions: ["write"] },
    (method) => ({
        create: method.create("write"),
        get: method.get("write"),
        delete: method.delete("write"),
    }),
    { package: NOTES },
);

/** Notes a branch drafts before the main line has them. */
export const note = defineObject(
    {
        name: "note",
        plural: "notes",
        scope: space,
        fields: {
            author: field.reference(principal.user).caller(),
            title: field.string(schema.string().min(1)),
        },
        permissions: { read: relation("author"), write: relation("author") },
    },
    (method) => ({
        create: method.create("write", { fields: ["title"] }),
    }),
    { package: NOTES },
);

/** A space's notes as objects of another package sharing the space database. */
export const sharedNote = defineObject({
    name: "shared-note",
    plural: "sharedNotes",
    scope: space,
    fields: { body: field.string() },
    permissions: ["read"],
});
