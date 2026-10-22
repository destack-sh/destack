import { MemoryKeychain } from "@destack/identity";
import { Identifier, present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { SessionStore } from "./session.ts";

test("save the first of two racing sign-ins' sessions for both, and replace it on a later sign-in", async () => {
    // store two sessions of one person at once and a third after both settled
    const keychain = new MemoryKeychain();
    const sessions = new SessionStore(keychain);
    const person = {
        issuer: "https://accounts.destack.test",
        subjectId: Identifier.create("user"),
    };
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
    const sessions = new SessionStore(new MemoryKeychain());
    const person = {
        issuer: "https://accounts.destack.test",
        subjectId: Identifier.create("user"),
    };
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

test("refresh a session's access token once for every caller at once, and anew once the person signs out", async () => {
    // ask for the account token twice at once, as two requests of one person do, then once more
    const sessions = new SessionStore(new MemoryKeychain());
    const person = {
        issuer: "https://accounts.destack.test",
        subjectId: Identifier.create("user"),
    };
    await sessions.replace(person, "session-a");
    let refreshed = 0;
    const refresh = async () => {
        refreshed += 1;

        return { accessToken: `access-${refreshed}`, expiresAt: Date.now() + 60_000 };
    };
    const tokens = await Promise.all([
        sessions.accessTokens(person).token("account", Date.now(), refresh),
        sessions.accessTokens(person).token("account", Date.now(), refresh),
    ]);
    const cached = await sessions.accessTokens(person).token("account", Date.now(), refresh);

    // forget the session as signing out does, and refresh again for the next caller
    await sessions.forget(person);
    const renewed = await sessions.accessTokens(person).token("account", Date.now(), refresh);

    expect({ tokens, cached, renewed, refreshed }).toEqual({
        tokens: ["access-1", "access-1"],
        cached: "access-1",
        renewed: "access-2",
        refreshed: 2,
    });
});
