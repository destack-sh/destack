import { expect, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { TABLE } from "@destack/db";
import { schema } from "@destack/schema";
import { defineObject, field } from "../src/index.ts";
import { space } from "./fixture/space.ts";

/** Contacts whose email only their owner reads, required when written. */
const contact = defineObject({
    name: "contact",
    plural: "contacts",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        name: field.string(),
        email: field.string().guard({ read: "update" }),
        isVerified: field.boolean().default(false).guard({ read: "update" }),
    },
    permissions: { read: relation("owner"), update: relation("owner") },
    methods: (method) => ({
        create: method.create("update", { fields: ["name", "email", "isVerified"] }),
    }),
});

test("require a guarded field on write, let readers miss it, and let copies keep it concealed", () => {
    // validate creations with and without the guarded fields
    const written = schema.object(contact.schema.written(["name", "email", "isVerified"], false));
    const creation = (value: object) => written.safeParse(value).success;
    const columns = contact.table[TABLE].columns;
    expect({
        withEmail: creation({ name: "Ada", email: "ada@example.com" }),
        withoutEmail: creation({ name: "Ada" }),
        concealed: contact.schema.row.pick({ email: true }).safeParse({}).success,
        nullable: [columns.email.definition.nullable, columns.isVerified.definition.nullable],
    }).toEqual({ withEmail: true, withoutEmail: false, concealed: true, nullable: [true, true] });
});
