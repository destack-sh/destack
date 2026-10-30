import { testWorld } from "../test/browser.ts";
import { expect, test } from "@destack/test";
import { createEffect, createRoot, createSignal, flush } from "solid-js";
import {
    createScheduled,
    debounce,
    leading,
    leadingAndTrailing,
    type ScheduleCallback,
    scheduleIdle,
    throttle,
} from "./scheduled.ts";

test("debounce to the latest call's arguments after the wait, or nothing once cleared", async () => {
    const { simulation, tab } = testWorld();
    // call twice, then once more and clear
    const values: number[] = [];
    const { trigger, dispose } = createRoot((disposeRoot) => ({
        trigger: debounce((value: number) => values.push(value), 20, tab.clock),
        dispose: disposeRoot,
    }));
    trigger(5);
    trigger(1);
    await simulation.clock.sleep(50);
    await simulation.idle();
    trigger(7);
    trigger.clear();
    await simulation.clock.sleep(50);
    await simulation.idle();
    dispose();

    expect(values).toEqual([1]);
});

test("throttle to the latest call's arguments, cancelled when cleared or cleaned up", async () => {
    const { simulation, tab } = testWorld();
    // call twice, then clear a call, then clean one up with its root
    const values: number[] = [];
    const trigger = throttle((value: number) => values.push(value), 20, tab.clock);
    trigger(5);
    trigger(1);
    await simulation.clock.sleep(50);
    await simulation.idle();
    trigger(7);
    trigger.clear();
    await simulation.clock.sleep(50);
    await simulation.idle();
    createRoot((disposeRoot) => {
        throttle((value: number) => values.push(value), 20, tab.clock)(9);
        disposeRoot();
    });
    await simulation.clock.sleep(50);
    await simulation.idle();

    expect(values).toEqual([1]);
});

test("run the leading edge of a debounce or throttle, again once the wait passes", async () => {
    const { simulation, tab } = testWorld();
    // call a leading debounce and a leading throttle three times around a wait
    const observed: number[][] = [];
    for (const schedule of [debounce, throttle] satisfies ScheduleCallback[]) {
        const values: number[] = [];
        const trigger = leading(schedule, (value: number) => values.push(value), 20, tab.clock);
        trigger(5);
        trigger(10);
        await simulation.clock.sleep(50);
        await simulation.idle();
        trigger(15);
        trigger.clear();
        trigger(20);
        observed.push(values);
    }

    expect(observed).toEqual([
        [5, 15, 20],
        [5, 15, 20],
    ]);
});

test("run the leading edge, and the trailing edge only when called again during the wait", async () => {
    const { simulation, tab } = testWorld();
    const observed: { calls: number; values: number[] }[] = [];
    for (const schedule of [debounce, throttle] satisfies ScheduleCallback[]) {
        // call once, which runs once
        let calls = 0;
        const single = leadingAndTrailing(schedule, () => calls++, 10, tab.clock);
        single();
        await simulation.clock.sleep(30);
        await simulation.idle();

        // call several times within the wait, which runs both edges
        const values: number[] = [];
        const trigger = leadingAndTrailing(
            schedule,
            (value: number) => values.push(value),
            20,
            tab.clock,
        );
        trigger(5);
        trigger(1);
        trigger(10);
        await simulation.clock.sleep(25);
        await simulation.idle();
        trigger(15);
        trigger.clear();
        trigger(20);
        observed.push({ calls, values });
    }

    expect(observed).toEqual([
        { calls: 1, values: [5, 10, 15, 20] },
        { calls: 1, values: [5, 10, 15, 20] },
    ]);
});

test("restart a leading and trailing debounce while calls keep coming", async () => {
    const { simulation, tab } = testWorld();
    const values: number[] = [];
    const trigger = leadingAndTrailing(
        debounce,
        (value: number) => values.push(value),
        20,
        tab.clock,
    );
    trigger(5);
    trigger(1);
    await simulation.clock.sleep(15);
    await simulation.idle();
    trigger(1);
    trigger(10);
    const during = [...values];
    await simulation.clock.sleep(25);
    await simulation.idle();

    expect([during, values]).toEqual([[5], [5, 10]]);
});

test("run a leading schedule on each call after its root is disposed", async () => {
    const { tab } = testWorld();
    const values: number[] = [];
    createRoot((disposeRoot) => {
        const first = leading(throttle, (value: number) => values.push(value), 150, tab.clock);
        const second = leadingAndTrailing(
            debounce,
            (value: number) => values.push(value),
            150,
            tab.clock,
        );
        first(5);
        second(6);
        disposeRoot();
        first(10);
        second(11);
    });

    expect(values).toEqual([5, 6, 10, 11]);
});

test("run an idle schedule once per idle period with the latest call's arguments, or nothing once cleared", async () => {
    // call twice before an idle period, then once more and clear
    const { simulation, tab } = testWorld();
    const values: number[] = [];
    const trigger = scheduleIdle((value: number) => values.push(value), 100, tab.display);
    trigger(5);
    trigger(1);
    await simulation.clock.sleep(1);
    await simulation.idle();
    trigger(7);
    trigger.clear();
    await simulation.clock.sleep(1);
    await simulation.idle();

    expect(values).toEqual([1]);
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

test("gate computations on a debounce, or on its leading edge", async () => {
    const { simulation, tab } = testWorld();
    // gate one effect on a debounce and one on a leading debounce
    const [track, trigger] = createSignal(undefined, { equals: false });
    const trailing: boolean[] = [];
    const first: boolean[] = [];
    const dispose = createRoot((disposeRoot) => {
        const debounced = createScheduled((callback) => debounce(callback, 20, tab.clock));
        const leadingEdge = createScheduled((callback) =>
            leading(debounce, callback, 20, tab.clock),
        );
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
    await simulation.clock.sleep(50);
    await simulation.idle();
    flush();
    dispose();

    expect([trailing, first]).toEqual([
        [false, false, true],
        [true, false],
    ]);
});

test("cancel a pending debounce of a scheduled gate when its root is disposed", async () => {
    const { simulation, tab } = testWorld();
    const [track, trigger] = createSignal(undefined, { equals: false });
    const values: boolean[] = [];
    const dispose = createRoot((disposeRoot) => {
        const scheduled = createScheduled((callback) => debounce(callback, 10_000, tab.clock));
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
    await simulation.clock.sleep(20_000);
    await simulation.idle();

    expect(values).toEqual([false, false]);
});
