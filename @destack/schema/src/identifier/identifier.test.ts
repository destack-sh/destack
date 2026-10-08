import { expect, test } from "@destack/test";
import { schema } from "../index.ts";
import { anyIdentifier, Identifier, identifier } from "./identifier.ts";

/** The UUIDv7 the fixture identifiers end with. */
const UUID = "01996ab0-0000-7000-8000-000000000001";

test("accept prefixed UUIDv7 identifiers of their own prefix only, and read their UUID", () => {
    // accept the prefix's identifiers and refuse others
    const space = identifier("space");
    expect(space.safeParse(`space-${UUID}`).success).toBe(true);
    expect(
        [`vault-${UUID}`, `space-${UUID.toUpperCase()}`, `space-${UUID}\n`, UUID].map(
            (value) => space.safeParse(value).success,
        ),
    ).toEqual([false, false, false, false]);

    // accept any valid prefix, and read the UUID back
    expect(anyIdentifier().safeParse(`role-permission-${UUID}`).success).toBe(true);
    expect(Identifier.uuid(space.parse(`space-${UUID}`))).toBe(UUID);
});

test("refuse prefixes that would make identifiers ambiguous", () => {
    for (const prefix of ["Space", "space-", "1space", "space--key", "space\n"]) {
        expect(() => identifier(prefix)).toThrow(
            new TypeError(`invalid identifier prefix: ${prefix}`),
        );
    }
});

test("derive the same UUIDv7 identifier for every repeat of a created or derived request", () => {
    // keep the time and bits of each request, marking them version 7
    const created = "01996ab0-0000-7000-8000-000000000001";
    const derived = "01996ab0-0000-8abc-9000-000000000002";
    expect([
        Identifier.derive("message", created),
        Identifier.derive("message", derived),
        Identifier.derive("message", derived),
    ]).toEqual([
        `message-${created}`,
        "message-01996ab0-0000-7abc-9000-000000000002",
        "message-01996ab0-0000-7abc-9000-000000000002",
    ]);

    // refuse a request identifier that is no UUID
    expect(() => Identifier.derive("message", "request-1")).toThrow(schema.Error);
});
