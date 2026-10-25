import { testHost } from "@destack/test/host";
import { Uuid } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { Simulation } from "@destack/environment/simulation";
import { RequestId } from "./request.ts";

/** A reading's time, 2026-10-06 in Unix milliseconds. */
const TIME = Date.UTC(2026, 9, 6);

test("derive the same time-ordered UUIDv8 for a key however often, its retry period read from its time", async () => {
    // derive a key twice at one time, another key at the same time, and the key a millisecond later
    const derived = await RequestId.derive(TIME, "stream-1 batch-1", testHost().crypto);
    const repeated = await RequestId.derive(TIME, "stream-1 batch-1", testHost().crypto);
    const other = await RequestId.derive(TIME, "stream-1 batch-2", testHost().crypto);
    const later = await RequestId.derive(TIME + 1, "stream-1 batch-1", testHost().crypto);

    // place the time in the leading 48 bits and the key's digest in the rest
    expect({
        derived,
        isRepeated: repeated === derived,
        isOther: other !== derived && other.slice(0, 13) === derived.slice(0, 13),
        isOrdered: derived < later && later.slice(14) === derived.slice(14),
        isAccepted: RequestId.schema.safeParse(derived).success,
        expiresAt: RequestId.expiry(derived, TIME),
    }).toEqual({
        derived: "01a10e82-9000-8585-a59a-5654bf8ac940",
        isRepeated: true,
        isOther: true,
        isOrdered: true,
        isAccepted: true,
        expiresAt: TIME + 7 * 24 * 60 * 60 * 1000,
    });
});

test("key derived identifiers of one key at different times as one request, and created ones whole", async () => {
    // derive one key at two times and another key at the first time, and create one in a world
    const world = new Simulation({ seed: 1, start: TIME }).environment("client");
    const first = await RequestId.derive(TIME, "trigger-1 event-1", testHost().crypto);
    const later = await RequestId.derive(TIME + 60_000, "trigger-1 event-1", testHost().crypto);
    const other = await RequestId.derive(TIME, "trigger-1 event-2", testHost().crypto);
    const created = RequestId.create(world);

    expect({
        first: RequestId.key(first),
        later: RequestId.key(later),
        other: RequestId.key(other),
        isCreatedWhole: RequestId.key(created) === created,
    }).toEqual({
        first: first.slice(14),
        later: first.slice(14),
        other: other.slice(14),
        isCreatedWhole: true,
    });
    expect(RequestId.key(other)).not.toEqual(RequestId.key(first));
});

test("accept created and derived identifiers, and refuse other UUID versions and expired ones", async () => {
    // derive an identifier, and create one in a world
    const world = new Simulation({ seed: 1, start: TIME }).environment("client");
    const derived = await RequestId.derive(TIME, "reading", testHost().crypto);

    expect({
        created: RequestId.schema.safeParse(RequestId.create(world)).success,
        derived: RequestId.schema.safeParse(derived).success,
        random: RequestId.schema.safeParse(Uuid.v4(world.random)).success,
        expired: await refusal(
            Promise.resolve().then(() => RequestId.expiry(derived, TIME + 8 * 24 * 60 * 60 * 1000)),
        ),
    }).toEqual({
        created: true,
        derived: true,
        random: false,
        expired: [
            "PRECONDITION_FAILED",
            "EXPIRED",
            "request identifier is outside its retry period",
        ],
    });
});
