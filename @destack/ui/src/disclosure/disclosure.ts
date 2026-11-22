import type { Accessor } from "@destack/view";

/** Take a native disclosure's toggle, leaving a controlled one at its owner's state. */
export function followToggle(
    event: Event & { readonly currentTarget: HTMLDetailsElement },
    isOpen: Accessor<boolean>,
    setOpen: (open: boolean) => void,
    isControlled: boolean,
): void {
    // ignore a toggle to the state it already has
    const element = event.currentTarget;
    if (element.open === isOpen()) {
        return;
    }

    // record the change and show the owner's state until the owner takes it
    setOpen(element.open);
    if (isControlled) {
        element.open = isOpen();
    }
}

/** Keep a trigger's disclosure as it is while the trigger is disabled. */
export function refuseDisabled(trigger: HTMLElement, isDisabled: Accessor<boolean>): void {
    trigger.addEventListener("click", (event) => {
        if (isDisabled()) {
            event.preventDefault();
        }
    });
}
