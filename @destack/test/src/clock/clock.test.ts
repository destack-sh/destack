import { expect, test } from "vitest";
import { TestClock } from "./clock.ts";

test("follow the system's clock until frozen, then stand still, set and advance by hand", () => {
    // follow the system's clock while unset
    const clock = new TestClock();
    const before = Date.now();
    const followed = clock.now();
    const after = Date.now();

    // freeze it, then move it by hand
    const frozen = clock.freeze();
    const read = [clock.now(), clock.now()];
    clock.advance(1000);
    const advanced = clock.now();
    clock.set(5);

    expect({
        isFollowing: followed >= before && followed <= after,
        read,
        advanced,
        set: clock.now(),
        started: new TestClock(42).now(),
    }).toEqual({
        isFollowing: true,
        read: [frozen, frozen],
        advanced: frozen + 1000,
        set: 5,
        started: 42,
    });
});

test("release sleeps as the clock passes their deadlines, earliest first, and refuse aborted ones", async () => {
    // sleep at three deadlines of a stopped clock, one of them aborted
    const clock = new TestClock(0);
    const woken: string[] = [];
    const aborting = new AbortController();
    const late = clock.sleep(300).then(() => woken.push("late"));
    const early = clock.sleep(100).then(() => woken.push("early"));
    const aborted = clock.sleep(200, aborting.signal).catch((error: unknown) => error);
    aborting.abort(new Error("stopped"));

    // pass the early deadline, then both
    clock.advance(150);
    await early;
    const first = [...woken];
    clock.set(400);
    await late;

    expect({ first, woken, aborted: await aborted, now: await clock.sleep(0) }).toEqual({
        first: ["early"],
        woken: ["early", "late"],
        aborted: new Error("stopped"),
        now: undefined,
    });
});
