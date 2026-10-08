import { afterAll, beforeAll, beforeEach, expect, test, vi } from "@destack/test";
import { createEffect, createRoot, createSignal, flush } from "solid-js";
import {
    createScheduled,
    debounce,
    leading,
    leadingAndTrailing,
    type ScheduleCallback,
    throttle,
} from "./scheduled.ts";

beforeAll(() => {
    vi.useFakeTimers();
});

beforeEach(() => {
    vi.clearAllTimers();
});

afterAll(() => {
    vi.useRealTimers();
});

test("debounce to the latest call's arguments after the wait, or nothing once cleared", () => {
    // call twice, then once more and clear
    const values: number[] = [];
    createRoot((disposeRoot) => {
        const trigger = debounce((value: number) => values.push(value), 20);
        trigger(5);
        trigger(1);
        vi.advanceTimersByTime(50);
        trigger(7);
        trigger.clear();
        vi.advanceTimersByTime(50);
        disposeRoot();
    });

    expect(values).toEqual([1]);
});

test("throttle to the latest call's arguments, cancelled when cleared or cleaned up", () => {
    // call twice, then clear a call, then clean one up with its root
    const values: number[] = [];
    const trigger = throttle((value: number) => values.push(value), 20);
    trigger(5);
    trigger(1);
    vi.advanceTimersByTime(50);
    trigger(7);
    trigger.clear();
    vi.advanceTimersByTime(50);
    createRoot((disposeRoot) => {
        throttle((value: number) => values.push(value), 20)(9);
        disposeRoot();
    });
    vi.advanceTimersByTime(50);

    expect(values).toEqual([1]);
});

test("run the leading edge of a debounce or throttle, again once the wait passes", () => {
    // call a leading debounce and a leading throttle three times around a wait
    const observed = ([debounce, throttle] satisfies ScheduleCallback[]).map((schedule) => {
        const values: number[] = [];
        const trigger = leading(schedule, (value: number) => values.push(value), 20);
        trigger(5);
        trigger(10);
        vi.advanceTimersByTime(50);
        trigger(15);
        trigger.clear();
        trigger(20);

        return values;
    });

    expect(observed).toEqual([
        [5, 15, 20],
        [5, 15, 20],
    ]);
});

test("run the leading edge, and the trailing edge only when called again during the wait", () => {
    const observed = ([debounce, throttle] satisfies ScheduleCallback[]).map((schedule) => {
        // call once, which runs once
        let calls = 0;
        const single = leadingAndTrailing(schedule, () => calls++, 10);
        single();
        vi.advanceTimersByTime(30);

        // call several times within the wait, which runs both edges
        const values: number[] = [];
        const trigger = leadingAndTrailing(schedule, (value: number) => values.push(value), 20);
        trigger(5);
        trigger(1);
        trigger(10);
        vi.advanceTimersByTime(25);
        trigger(15);
        trigger.clear();
        trigger(20);

        return { calls, values };
    });

    expect(observed).toEqual([
        { calls: 1, values: [5, 10, 15, 20] },
        { calls: 1, values: [5, 10, 15, 20] },
    ]);
});

test("restart a leading and trailing debounce while calls keep coming", () => {
    const values: number[] = [];
    const trigger = leadingAndTrailing(debounce, (value: number) => values.push(value), 20);
    trigger(5);
    trigger(1);
    vi.advanceTimersByTime(15);
    trigger(1);
    trigger(10);
    const during = [...values];
    vi.advanceTimersByTime(25);

    expect([during, values]).toEqual([[5], [5, 10]]);
});

test("run a leading schedule on each call after its root is disposed", () => {
    const values: number[] = [];
    createRoot((disposeRoot) => {
        const first = leading(throttle, (value: number) => values.push(value), 150);
        const second = leadingAndTrailing(debounce, (value: number) => values.push(value), 150);
        first(5);
        second(6);
        disposeRoot();
        first(10);
        second(11);
    });

    expect(values).toEqual([5, 6, 10, 11]);
});

test("report a scheduled gate dirty once its schedule fires", () => {
    let invalidate: (() => void) | undefined;
    const scheduled = createScheduled((callback) => {
        invalidate = callback;

        return () => {};
    });
    const before = scheduled();
    invalidate?.();

    expect([before, scheduled()]).toEqual([false, true]);
});

test("rerun every computation tracking a scheduled gate when it fires", () => {
    // track the gate from two effects
    let invalidate: (() => void) | undefined;
    const first: boolean[] = [];
    const second: boolean[] = [];
    const [track, trigger] = createSignal(undefined, { equals: false });
    const dispose = createRoot((disposeRoot) => {
        const scheduled = createScheduled((callback) => {
            invalidate = callback;

            return () => {};
        });
        for (const values of [first, second]) {
            createEffect(
                () => {
                    track();

                    return scheduled();
                },
                (value) => {
                    values.push(value);
                },
            );
        }

        return disposeRoot;
    });
    flush();

    // rerun clean on other changes, and dirty once the schedule fires
    trigger(undefined);
    flush();
    invalidate?.();
    flush();
    dispose();

    expect([first, second]).toEqual([
        [false, false, true],
        [false, false, true],
    ]);
});

test("gate computations on a debounce, or on its leading edge", () => {
    // gate one effect on a debounce and one on a leading debounce
    const [track, trigger] = createSignal(undefined, { equals: false });
    const trailing: boolean[] = [];
    const first: boolean[] = [];
    const dispose = createRoot((disposeRoot) => {
        const debounced = createScheduled((callback) => debounce(callback, 20));
        const leadingEdge = createScheduled((callback) => leading(debounce, callback, 20));
        createEffect(
            () => {
                track();

                return debounced();
            },
            (value) => {
                trailing.push(value);
            },
        );
        createEffect(
            () => {
                track();

                return leadingEdge();
            },
            (value) => {
                first.push(value);
            },
        );

        return disposeRoot;
    });
    flush();
    trigger(undefined);
    flush();
    vi.advanceTimersByTime(50);
    flush();
    dispose();

    expect([trailing, first]).toEqual([
        [false, false, true],
        [true, false],
    ]);
});

test("cancel a pending debounce of a scheduled gate when its root is disposed", () => {
    const [track, trigger] = createSignal(undefined, { equals: false });
    const values: boolean[] = [];
    const dispose = createRoot((disposeRoot) => {
        const scheduled = createScheduled((callback) => debounce(callback, 10_000));
        createEffect(
            () => {
                track();

                return scheduled();
            },
            (value) => {
                values.push(value);
            },
        );

        return disposeRoot;
    });
    flush();
    trigger(undefined);
    flush();
    dispose();
    vi.advanceTimersByTime(20_000);

    expect(values).toEqual([false, false]);
});
