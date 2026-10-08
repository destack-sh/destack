import { afterAll, beforeAll, expect, test, vi } from "@destack/test";
import { debounce, leading, leadingAndTrailing, scheduleIdle, throttle } from "./scheduled.ts";

beforeAll(() => {
    vi.useFakeTimers();
});

afterAll(() => {
    vi.useRealTimers();
});

test("run nothing scheduled on the server, and only the first leading call", () => {
    // trigger every schedule twice around all timers
    const called: string[] = [];
    const triggers = [
        debounce((value: string) => called.push(`debounce ${value}`), 100),
        throttle((value: string) => called.push(`throttle ${value}`), 100),
        scheduleIdle((value: string) => called.push(`idle ${value}`), 100),
        leading(debounce, (value: string) => called.push(`leading ${value}`), 100),
        leadingAndTrailing(debounce, (value: string) => called.push(`both ${value}`), 100),
    ];
    for (const trigger of triggers) {
        trigger("foo");
    }
    vi.runAllTimers();
    for (const trigger of triggers) {
        trigger("baz");
    }

    expect(called).toEqual(["leading foo", "both foo"]);
});
