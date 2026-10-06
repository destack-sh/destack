import type { Accessor } from "@destack/view";

/** Take a native disclosure's toggle, holding a controlled one at the state its owner keeps. */
export function followToggle(
    event: Event & { readonly currentTarget: HTMLDetailsElement },
    isOpen: Accessor<boolean>,
    setOpen: (open: boolean) => void,
    isControlled: boolean,
): void {
    // ignore a toggle the state already holds
    const element = event.currentTarget;
    if (element.open === isOpen()) {
        return;
    }

    // keep the change and show the owner's state until the owner takes it
    setOpen(element.open);
    if (isControlled) {
        element.open = isOpen();
    }
}

/** Keep a trigger's disclosure as it is while the trigger is disabled, listening on the trigger before the platform toggles. */
export function refuseDisabled(trigger: HTMLElement, isDisabled: Accessor<boolean>): void {
    trigger.addEventListener("click", (event) => {
        if (isDisabled()) {
            event.preventDefault();
        }
    });
}
