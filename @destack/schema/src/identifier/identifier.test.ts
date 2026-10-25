import { TEST_START, testSimulation } from "@destack/test/world";
import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { schema } from "../index.ts";
import { anyIdentifier, Identifier, identifier } from "./identifier.ts";

/** The UUIDv7 the fixture identifiers end with. */
const UUID = "01996ab0-0000-7000-8000-000000000001";

/** Create two notes in the first millisecond of a seeded world and one in the next. */
async function createNotes(seed: number): Promise<string[]> {
    // create two identifiers, advance the clock, and create a third
    await using simulation = new Simulation({ seed, start: TEST_START });
    const environment = simulation.environment("schema");
    const created = [
        Identifier.create("note", environment),
        Identifier.create("note", environment),
    ];
    simulation.clock.advance(1);

    return [...created, Identifier.create("note", environment)];
}

test("accept prefixed UUIDv7 identifiers of their own prefix only, and read their UUID", () => {
    // accept the prefix's identifiers and refuse others
    const space = identifier("space");
    expect(space.safeParse(`space-${UUID}`).success).toBe(true);
    expect(
        [`vault-${UUID}`, `space-${UUID.toUpperCase()}`, `space-${UUID}\n`, UUID].map(
            (value) => space.safeParse(value).success,
        ),
    ).toEqual([false, false, false, false]);

    // accept any valid prefix, and read the UUID and the prefix back
    expect(anyIdentifier().safeParse(`role-permission-${UUID}`).success).toBe(true);
    expect(Identifier.uuid(space.parse(`space-${UUID}`))).toBe(UUID);
    expect(Identifier.prefix(anyIdentifier().parse(`role-permission-${UUID}`))).toBe(
        "role-permission",
    );
});

test("filter the identifiers of one prefix from mixed values in their order", () => {
    const space = `space-${UUID}`;
    const account = `account-${UUID}`;
    expect(Identifier.filter("space", [account, space, "space", undefined, space])).toEqual([
        space,
        space,
    ]);
});

test("refuse prefixes that would make identifiers ambiguous", () => {
    for (const prefix of ["Space", "space-", "1space", "space--key", "space\n"]) {
        expect(() => identifier(prefix)).toThrow(
            new TypeError(`invalid identifier prefix: ${prefix}`),
        );
    }
});

test("derive the same UUIDv7 identifier for every repeat of a created or derived request", () => {
    // take the time and bits of each request, marking them version 7
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

    // refuse a request identifier that is not a UUID
    expect(() => Identifier.derive("message", "request-1")).toThrow(schema.Error);
});

test("derive a request's objects in order of their positions, between requests whose bits differ before the position", () => {
    // write each position over bytes 11 to 13 of a request's bits
    const earlier = "01996ab0-0000-7000-8000-00ffffffffff";
    const request = "01996ab0-0000-7000-8000-01abcdefabcd";
    const later = "01996ab0-0000-7000-8000-020000000000";
    const derived = [
        Identifier.derive("task", earlier, 70_000),
        Identifier.derive("task", request, 0),
        Identifier.derive("task", request, 1),
        Identifier.derive("task", request, 70_000),
        Identifier.derive("task", later, 0),
    ];
    expect(derived).toEqual([
        "task-01996ab0-0000-7000-8000-00011170ffff",
        "task-01996ab0-0000-7000-8000-01000000abcd",
        "task-01996ab0-0000-7000-8000-01000001abcd",
        "task-01996ab0-0000-7000-8000-01011170abcd",
        "task-01996ab0-0000-7000-8000-020000000000",
    ]);

    // sort them as they were derived, and refuse a position past 2^24
    expect(derived.toSorted()).toEqual(derived);
    expect(() => Identifier.derive("task", request, 2 ** 24)).toThrow(
        new RangeError("identifier position 16777216 is outside 0 to 2^24"),
    );
});

test("create the same identifiers in two worlds of one seed, ordered within each millisecond", async () => {
    // create two identifiers per millisecond in a world of a seed
    const [first, again, other] = [
        await createNotes(7),
        await createNotes(7),
        await createNotes(8),
    ];

    // repeat identifiers for one seed
    const seeded = [
        "note-01a0f4c2-c400-72ed-b523-b5e148fc2bbd",
        "note-01a0f4c2-c400-72ee-b42d-75fe2d705834",
        "note-01a0f4c2-c401-7093-afd3-b1ae7e86dad9",
    ];
    expect({ first, again, other }).toEqual({
        first: seeded,
        again: seeded,
        other: [
            "note-01a0f4c2-c400-774a-9c2b-b84c515e5561",
            "note-01a0f4c2-c400-774b-bab4-2ef1b2e496de",
            "note-01a0f4c2-c401-7238-849f-eebbf46a8dbf",
        ],
    });
});

test("hash a name in a namespace into the same UUIDv8 identifier everywhere, accepted as its prefix's", async () => {
    // hash an installation's name twice, and another name once
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("schema");
    const hash = (name: string) => Identifier.hash("installation", UUID, name, crypto);
    const hashed = [await hash("stack"), await hash("stack"), await hash("forge")];

    expect([...hashed, identifier("installation").safeParse(hashed[0]).success]).toEqual([
        "installation-1b1d1b61-3477-89e0-9505-ddb2f256bf66",
        "installation-1b1d1b61-3477-89e0-9505-ddb2f256bf66",
        "installation-eeb98c34-92fe-85b9-9ba0-3e7944b02006",
        true,
    ]);
});
