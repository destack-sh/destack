import { type BeforeLeaveEventArgs, type Location, useBeforeLeave } from "@solidjs/router";
import { type Accessor, createSignal } from "solid-js";

/** Whether a blocker holds a navigation for the person to confirm. */
export type BlockerState = "unblocked" | "blocked";

/** How a navigation moves through history: back or forward, a new entry, or in place of the current one. */
export type HistoryAction = "POP" | "PUSH" | "REPLACE";

/** Decide whether to block a navigation from where the page is, to where it goes, and how it moves through history. */
export type BlockerFunction = (args: {
    /** Where the page is. */
    readonly currentLocation: Location;
    /** The path or history step the navigation goes to. */
    readonly nextLocation: string | number;
    /** How the navigation moves through history. */
    readonly historyAction: HistoryAction;
}) => boolean;

/** A navigation blocker: whether it holds a navigation, where that goes, and how to let it through or drop it. */
export interface Blocker {
    /** Whether a navigation is held. */
    readonly state: Accessor<BlockerState>;
    /** Where the held navigation goes, absent while none is held. */
    readonly location: Accessor<string | number | undefined>;
    /** Let the held navigation through. */
    proceed(): void;
    /** Drop the held navigation and stay. */
    reset(): void;
}

/** Hold navigations within the page while told to, so the page can ask the person before leaving, until cleanup. */
export function useBlocker(shouldBlock: boolean | BlockerFunction): Blocker {
    // hold the navigation waiting for the person
    const [held, setHeld] = createSignal<BeforeLeaveEventArgs | undefined>(undefined, {
        ownedWrite: true,
    });

    // hold each navigation the condition blocks, unless another listener already did
    useBeforeLeave((event) => {
        // weigh the navigation, leaving one an earlier listener held
        const isBlocked =
            typeof shouldBlock === "function"
                ? shouldBlock({
                      currentLocation: event.from,
                      nextLocation: event.to,
                      historyAction: historyActionOf(event),
                  })
                : shouldBlock;
        if (!isBlocked || event.defaultPrevented) {
            return;
        }
        event.preventDefault();
        setHeld(() => event);
    });

    return {
        state: () => (held() === undefined ? "unblocked" : "blocked"),
        location: () => held()?.to,
        proceed: () => {
            // let the held navigation through past every blocker
            const event = held();
            if (event === undefined) {
                throw new TypeError("no navigation is blocked");
            }
            setHeld(undefined);
            event.retry(true);
        },
        reset: () => {
            setHeld(undefined);
        },
    };
}

/** Read how a navigation moves through history: a step back or forward, in place, or a new entry. */
function historyActionOf(event: BeforeLeaveEventArgs): HistoryAction {
    if (typeof event.to === "number") {
        return "POP";
    } else if (event.options?.replace === true) {
        return "REPLACE";
    }

    return "PUSH";
}
