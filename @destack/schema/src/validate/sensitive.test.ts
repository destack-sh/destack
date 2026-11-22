import { expect, test } from "@destack/test";
import * as schema from "./index.ts";

test("leave out sensitive values through objects, arrays and wrappers, handing each to the finder", () => {
    // mark a password and a list of tokens sensitive
    const login = schema.object({
        name: schema.string(),
        password: schema.sensitive(schema.string()),
        tokens: schema.array(schema.sensitive(schema.string())).exactOptional(),
        recovery: schema.nullable(schema.sensitive(schema.string())),
    });

    // drop each sensitive value, leaving array positions as null
    const found: unknown[] = [];
    expect(
        new schema.Redaction(login).redact(
            { name: "ada", password: "secret", tokens: ["a", "b"], recovery: null },
            (value) => found.push(value),
        ),
    ).toEqual({ name: "ada", tokens: [null, null], recovery: null });
    expect(found).toEqual(["secret", "a", "b"]);
});

test("map personal values apart from secrets, through the same walk that leaves secrets out", () => {
    // mark an address personal and a token secret
    const call = schema.object({
        address: schema.sensitive(schema.string(), "personal"),
        token: schema.sensitive(schema.string()),
        method: schema.string(),
    });

    // seal the personal value and drop the secret
    expect([
        schema.sensitivityOf(call.shape.address),
        new schema.Redaction(call).map(
            { address: "ada@example.com", token: "t", method: "update" },
            (value, sensitivity) => (sensitivity === "personal" ? { sealed: value } : undefined),
        ),
        new schema.Redaction(call).redact({
            address: "ada@example.com",
            token: "t",
            method: "update",
        }),
    ]).toEqual([
        "personal",
        { address: { sealed: "ada@example.com" }, method: "update" },
        { method: "update" },
    ]);
});

test("mark a copy of a shared schema, leaving the original and its other uses plain", () => {
    // mark a shared schema personal
    const attributes = schema.record(schema.string(), schema.string());
    const personal = schema.sensitive(attributes, "personal");

    // read the mark on the copy, its clones and the original
    expect([
        personal === attributes,
        schema.sensitivityOf(personal),
        schema.isSensitive(personal),
        schema.sensitivityOf(personal.describe("the personal attributes")),
        schema.isSensitive(personal.describe("the personal attributes")),
        schema.sensitivityOf(attributes),
        schema.isSensitive(attributes),
    ]).toEqual([false, "personal", true, "personal", true, undefined, false]);

    // leave values under the original and drop them under the copy
    const value = { "enduser.email": "ada@example.com" };
    expect([
        new schema.Redaction(attributes).redact(value),
        new schema.Redaction(personal).redact(value),
    ]).toEqual([value, undefined]);
});

test("mask the personal values a schema marks at any depth, leaving the shape, secrets and other values", () => {
    // mark an address, a list of names and a record of attributes personal, and a token secret
    const call = schema.object({
        method: schema.string(),
        context: schema.object({ address: schema.sensitive(schema.string(), "personal") }),
        names: schema.sensitive(schema.array(schema.string()), "personal"),
        attributes: schema.sensitive(schema.record(schema.string(), schema.string()), "personal"),
        token: schema.sensitive(schema.string()),
    });
    const plain = schema.object({
        method: schema.string(),
        token: schema.sensitive(schema.string()),
    });

    // mask each personal value entry by entry, and report the schemas marking any
    expect([
        new schema.Redaction(call).isPersonal,
        new schema.Redaction(plain).isPersonal,
        new schema.Redaction(schema.lazy(() => call)).isPersonal,
        new schema.Redaction(call).mask({
            method: "update",
            context: { address: "ada@example.com" },
            names: ["ada", "grace"],
            attributes: { city: "London" },
            token: "t",
        }),
    ]).toEqual([
        true,
        false,
        true,
        {
            method: "update",
            context: { address: "****" },
            names: ["****", "****"],
            attributes: { city: "****" },
            token: "t",
        },
    ]);
});

test("leave out secrets and mask personal values inside records, tuples, unions and intersections", () => {
    // declare secrets and personal values nested in each composite schema
    const settings = schema.object({
        keys: schema.record(schema.string(), schema.sensitive(schema.string())),
        pair: schema.tuple([schema.string(), schema.sensitive(schema.string())]),
        contact: schema.union([
            schema.object({
                kind: schema.literal("email"),
                email: schema.sensitive(schema.string(), "personal"),
            }),
            schema.object({ kind: schema.literal("none") }),
        ]),
        owner: schema.intersection(
            schema.object({ name: schema.sensitive(schema.string(), "personal") }),
            schema.object({ role: schema.string() }),
        ),
    });
    const value = {
        keys: { stripe: "sk", github: "gh" },
        pair: ["id", "secret"],
        contact: { kind: "email", email: "ada@example.com" },
        owner: { name: "Ada", role: "admin" },
    };

    // report the marks, leave out every sensitive value, and mask only the personal ones
    expect([
        new schema.Redaction(settings).isPersonal,
        new schema.Redaction(schema.object({ keys: settings.shape.keys })).isPersonal,
        new schema.Redaction(settings).redact(value),
        new schema.Redaction(settings).mask(value),
    ]).toEqual([
        true,
        false,
        { keys: {}, pair: ["id", null], contact: { kind: "email" }, owner: { role: "admin" } },
        {
            keys: { stripe: "sk", github: "gh" },
            pair: ["id", "secret"],
            contact: { kind: "email", email: "****" },
            owner: { name: "****", role: "admin" },
        },
    ]);
});

test("mask by every union option a value matches, and refuse a value matching none", () => {
    // let a plain option parse the email value too, dropping the field it does not know
    const contact = schema.union([
        schema.object({ kind: schema.string() }),
        schema.object({
            kind: schema.literal("email"),
            email: schema.sensitive(schema.string(), "personal"),
        }),
    ]);

    // mask the email though the plain option matches first, and refuse a number
    expect([
        new schema.Redaction(contact).mask({ kind: "email", email: "ada@example.com" }),
        (() => {
            try {
                return new schema.Redaction(contact).mask(1);
            } catch (error) {
                return error instanceof TypeError ? error.message : "thrown";
            }
        })(),
    ]).toEqual([{ kind: "email", email: "****" }, "a value matches no option of its union"]);
});

test("leave out secrets at every depth of recursive schemas, through lazy schemas and getters", () => {
    // declare a tree through a lazy schema and a thread through a getter, each with a secret per node
    const tree: schema.Schema<{ token: string; children: unknown[] }> = schema.object({
        token: schema.sensitive(schema.string()),
        children: schema.array(schema.lazy(() => tree)),
    });
    const thread = schema.object({
        token: schema.sensitive(schema.string()),
        get reply() {
            return thread.exactOptional();
        },
    });
    const plain: schema.Schema<{ children: unknown[] }> = schema.object({
        children: schema.array(schema.lazy(() => plain)),
    });

    // drop each node's secret, and leave a recursive schema marking nothing as it is
    const nested = { children: [{ children: [] }] };
    expect([
        new schema.Redaction(tree).redact({
            token: "a",
            children: [{ token: "b", children: [{ token: "c", children: [] }] }],
        }),
        new schema.Redaction(thread).redactFields({ token: "a", reply: { token: "b" } }),
        new schema.Redaction(plain).redact(nested) === nested,
        new schema.Redaction(tree).isPersonal,
    ]).toEqual([{ children: [{ children: [{ children: [] }] }] }, { reply: {} }, true, false]);
});
