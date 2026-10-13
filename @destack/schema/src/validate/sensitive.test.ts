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

    // drop each sensitive value, keeping array positions as null
    const found: unknown[] = [];
    expect(
        schema.redact(
            login,
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
        schema.mapSensitive(
            call,
            { address: "ada@example.com", token: "t", method: "update" },
            (value, sensitivity) => (sensitivity === "personal" ? { sealed: value } : undefined),
        ),
        schema.redact(call, { address: "ada@example.com", token: "t", method: "update" }),
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

    // keep values under the original and drop them under the copy
    const value = { "enduser.email": "ada@example.com" };
    expect([schema.redact(attributes, value), schema.redact(personal, value)]).toEqual([
        value,
        undefined,
    ]);
});

test("mask the personal values a schema marks at any depth, keeping the shape, secrets and other values", () => {
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
        schema.marksPersonal(call),
        schema.marksPersonal(plain),
        schema.marksPersonal(schema.lazy(() => call)),
        schema.mask(call, {
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

    // report the marks, leave out every sensitive value, and mask the personal ones alone
    expect([
        schema.marksPersonal(settings),
        schema.marksPersonal(schema.object({ keys: settings.shape.keys })),
        schema.redact(settings, value),
        schema.mask(settings, value),
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
        schema.mask(contact, { kind: "email", email: "ada@example.com" }),
        (() => {
            try {
                return schema.mask(contact, 1);
            } catch (error) {
                return error instanceof TypeError ? error.message : "thrown";
            }
        })(),
    ]).toEqual([{ kind: "email", email: "****" }, "a value matches no option of its union"]);
});
