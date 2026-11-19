import { expect, test } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
import { createEventDispatcher } from "./event-dispatcher.ts";

/** The properties of a component that dispatches events. */
interface Properties {
    /** Handle a step change. */
    readonly onChangeStep: (event: CustomEvent<string>) => void;
    /** Handle a cancellation. */
    readonly onCancel: (event: CustomEvent<null>) => void;
    /** Handle an event nobody listens to. */
    readonly onNullableEvent?: () => void;
    /** A property that handles nothing. */
    readonly nonEvent: string;
}

test("dispatch typed events to handlers, reporting cancellation", () => {
    const observed = createRoot((disposeRoot) => {
        // handle steps and cancel the cancellation
        const [step, setStep] = createSignal("first", { ownedWrite: true });
        const dispatch = createEventDispatcher<Properties>({
            onChangeStep: (event) => setStep(event.detail),
            onCancel: (event) => {
                event.preventDefault();
                setStep(`one after the ${step()}`);
            },
            nonEvent: "not an event",
        });

        // dispatch to a handler, to none, and to one that cancels
        const changed = dispatch("changeStep", "second");
        flush();
        const second = step();
        const unhandled = dispatch("nullableEvent");
        const cancelled = dispatch("cancel", null, { cancelable: true });
        flush();
        disposeRoot();

        return { changed, second, unhandled, cancelled, last: step() };
    });

    expect(observed).toEqual({
        changed: true,
        second: "second",
        unhandled: true,
        cancelled: false,
        last: "one after the second",
    });
});
