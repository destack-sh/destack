import { MemoryKeychain } from "@destack/identity";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { SessionKeychain } from "./session-keychain.ts";

test("keep the first of two racing sign-ins' sessions for both, and replace it on a later sign-in", async () => {
    // keep two sessions of one person at once and a third after both settled
    const keychain = new MemoryKeychain();
    const sessions = new SessionKeychain(keychain);
    const person = { issuer: "https://accounts.destack.test", subject: "user-1" };
    const raced = await Promise.all([
        sessions.keep(person, "session-a"),
        sessions.keep(person, "session-b"),
    ]);
    const later = await sessions.keep(person, "session-c");

    // adopt the first kept session in the racing sign-in and let the later one replace it
    expect([raced, later, await sessions.load(person)]).toEqual([
        ["session-a", "session-a"],
        "session-c",
        "session-c",
    ]);
});

test("forget a signed-out person's session, keeping the session a racing sign-in kept after sign-out read it", async () => {
    // read the session as sign-out does and keep a new one as a racing sign-in does
    const sessions = new SessionKeychain(new MemoryKeychain());
    const person = { issuer: "https://accounts.destack.test", subject: "user-1" };
    await sessions.keep(person, "session-a");
    const signedOut = present(await sessions.load(person), "the session sign-out read");
    await sessions.keep(person, "session-b");

    // keep the new session past the sign-out and forget it once signed out itself
    const raced = await sessions.remove(person, signedOut);
    const kept = await sessions.load(person);
    const removed = await sessions.remove(person, "session-b");
    expect([signedOut, raced, kept, removed, await sessions.load(person)]).toEqual([
        "session-a",
        false,
        "session-b",
        true,
        undefined,
    ]);
});
