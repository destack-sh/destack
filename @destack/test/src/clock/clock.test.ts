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
