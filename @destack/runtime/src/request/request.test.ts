import { expect, refusal, test } from "@destack/test";
import { copyRequest, RequestId } from "./request.ts";

/** A reading's time, 2026-10-06 in Unix milliseconds. */
const TIME = Date.UTC(2026, 9, 6);

test("derive the same time-ordered UUIDv8 for a key however often, its retry period read from its time", async () => {
    // derive a key twice at one time, another key at the same time, and the key a millisecond later
    const derived = await RequestId.derive(TIME, "stream-1 batch-1");
    const repeated = await RequestId.derive(TIME, "stream-1 batch-1");
    const other = await RequestId.derive(TIME, "stream-1 batch-2");
    const later = await RequestId.derive(TIME + 1, "stream-1 batch-1");

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
    // derive one key at two times and another key at the first time
    const first = await RequestId.derive(TIME, "trigger-1 event-1");
    const later = await RequestId.derive(TIME + 60_000, "trigger-1 event-1");
    const other = await RequestId.derive(TIME, "trigger-1 event-2");
    const created = RequestId.create(TIME);

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
    const derived = await RequestId.derive(TIME, "reading");
    expect({
        created: RequestId.schema.safeParse(RequestId.create(Date.now())).success,
        derived: RequestId.schema.safeParse(derived).success,
        random: RequestId.schema.safeParse(crypto.randomUUID()).success,
        expired: await refusal(
            Promise.resolve().then(() => RequestId.expiry(derived, TIME + 8 * 24 * 60 * 60 * 1000)),
        ),
    }).toEqual({
        created: true,
        derived: true,
        random: false,
        expired: ["PRECONDITION_FAILED", "request identifier is outside its retry period"],
    });
});

test("copy a request with its headers replaced, empty ones included", () => {
    // strip every header of a request, and replace one of another
    const sent = new Request("https://egress.test/a", {
        headers: { authorization: "Bearer secret", cookie: "c" },
    });
    const stripped = copyRequest(sent, { headers: new Headers() }, "https://api.test/b");
    const replaced = copyRequest(stripped, { headers: { accept: "text/plain" } });

    expect({
        stripped: [stripped.url, [...stripped.headers]],
        replaced: [...replaced.headers],
    }).toEqual({
        stripped: ["https://api.test/b", []],
        replaced: [["accept", "text/plain"]],
    });
});
