import { MemoryKeychain } from "@destack/identity";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { SessionKeychain } from "./session-keychain.ts";

test("save the first of two racing sign-ins' sessions for both, and replace it on a later sign-in", async () => {
    // store two sessions of one person at once and a third after both settled
    const keychain = new MemoryKeychain();
    const sessions = new SessionKeychain(keychain);
    const person = { issuer: "https://accounts.destack.test", subject: "user-1" };
    const raced = await Promise.all([
        sessions.replace(person, "session-a"),
        sessions.replace(person, "session-b"),
    ]);
    const later = await sessions.replace(person, "session-c");

    // adopt the first stored session in the racing sign-in and let the later one replace it
    expect([raced, later, await sessions.load(person)]).toEqual([
        ["session-a", "session-a"],
        "session-c",
        "session-c",
    ]);
});

test("forget a signed-out person's session, leaving the session a racing sign-in saved after sign-out read it", async () => {
    // read the session as sign-out does and store a new one as a racing sign-in does
    const sessions = new SessionKeychain(new MemoryKeychain());
    const person = { issuer: "https://accounts.destack.test", subject: "user-1" };
    await sessions.replace(person, "session-a");
    const signedOut = present(await sessions.load(person), "the session sign-out read");
    await sessions.replace(person, "session-b");

    // store the new session past the sign-out and forget it once signed out itself
    const raced = await sessions.remove(person, signedOut);
    const saved = await sessions.load(person);
    const removed = await sessions.remove(person, "session-b");
    expect([signedOut, raced, saved, removed, await sessions.load(person)]).toEqual([
        "session-a",
        false,
        "session-b",
        true,
        undefined,
    ]);
});
