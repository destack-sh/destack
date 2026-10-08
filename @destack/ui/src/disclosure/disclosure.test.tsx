import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import { followToggle, refuseDisabled } from "./index.ts";

/** Render a native disclosure that hands each toggle to a handler, and return it. */
function drawDetails(
    onToggle: (event: Event & { readonly currentTarget: HTMLDetailsElement }) => void,
): HTMLDetailsElement {
    const container = draw(() => (
        <details onToggle={onToggle}>
            <summary>Advanced</summary>
            Options
        </details>
    ));
    const details = container.querySelector("details");
    if (details === null) {
        throw new TypeError("the disclosure did not render");
    }

    return details;
}

/** Open a native disclosure as the platform does, firing its toggle. */
function open(details: HTMLDetailsElement): void {
    details.open = true;
    details.dispatchEvent(new Event("toggle"));
}

test("take the platform's toggle into an uncontrolled disclosure's own state", () => {
    let isOpen = false;
    const changes: boolean[] = [];
    const details = drawDetails((event) =>
        followToggle(
            event,
            () => isOpen,
            (next) => {
                changes.push(next);
                isOpen = next;
            },
            false,
        ),
    );
    open(details);

    expect({ isOpen, shown: details.open, changes }).toEqual({
        isOpen: true,
        shown: true,
        changes: [true],
    });
});

test("report the platform's toggle to a controlled disclosure's owner and show the owner's state", () => {
    const changes: boolean[] = [];
    const details = drawDetails((event) =>
        followToggle(
            event,
            () => false,
            (next) => changes.push(next),
            true,
        ),
    );
    open(details);

    expect({ shown: details.open, changes }).toEqual({ shown: false, changes: [true] });
});

test("keep a disclosure as it is while its trigger is disabled", () => {
    // click the trigger while disabled, then while enabled
    let isDisabled = true;
    const details = drawDetails(() => undefined);
    const summary = details.querySelector("summary");
    if (summary === null) {
        throw new TypeError("the trigger did not render");
    }
    refuseDisabled(summary, () => isDisabled);
    const refused = new MouseEvent("click", { bubbles: true, cancelable: true });
    summary.dispatchEvent(refused);
    isDisabled = false;
    const allowed = new MouseEvent("click", { bubbles: true, cancelable: true });
    summary.dispatchEvent(allowed);

    expect([refused.defaultPrevented, allowed.defaultPrevented]).toEqual([true, false]);
});
