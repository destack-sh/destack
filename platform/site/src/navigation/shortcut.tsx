import { onSettled } from "@destack/view";

/// Activate visible mnemonic controls through Alt plus their highlighted key.
export function KeyboardShortcuts() {
    onSettled(() => {
        const activate = (event: KeyboardEvent) => {
            if (
                event.defaultPrevented ||
                !event.altKey ||
                event.ctrlKey ||
                event.metaKey ||
                isEditable(event.target)
            ) {
                return;
            }

            // normalize the physical key before matching visible controls
            const shortcut = shortcutFor(event);
            if (shortcut === "j" || shortcut === "k") {
                event.preventDefault();
                moveFocus(shortcut === "j" ? 1 : -1);

                return;
            }

            if (event.repeat) {
                return;
            }

            const controls = document.querySelectorAll<HTMLElement>("[data-shortcut]");
            const control = [...controls].find(
                (candidate) => candidate.dataset.shortcut?.toLowerCase() === shortcut,
            );
            if (control == undefined) {
                return;
            }

            event.preventDefault();
            control.focus();
            control.click();
        };

        document.addEventListener("keydown", activate);
        return () => document.removeEventListener("keydown", activate);
    });

    return null;
}

/// Move focus through every visible interactive control in document order.
function moveFocus(direction: 1 | -1) {
    const scope = document.querySelector("dialog[open]") ?? document;
    const candidates = scope.querySelectorAll<HTMLElement>(
        "a[href], button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex='-1'])",
    );
    const controls = [...candidates].filter((candidate) => candidate.getClientRects().length > 0);
    if (controls.length === 0) {
        return;
    }

    // wrap around the active control, or begin at the corresponding edge
    const active = controls.indexOf(document.activeElement as HTMLElement);
    const base = active < 0 ? (direction === 1 ? -1 : 0) : active;
    const index = (base + direction + controls.length) % controls.length;
    controls[index].focus({ preventScroll: true });
    controls[index].scrollIntoView({ block: "nearest" });
}

/// Normalize physical letter and number keys across keyboard layouts and Option modifiers.
function shortcutFor(event: KeyboardEvent) {
    // preserve mnemonics when Option changes the produced character on macOS
    if (event.code.startsWith("Key")) {
        return event.code.slice(3).toLowerCase();
    }

    // preserve the number row independently of Shift and keyboard layout
    if (event.code.startsWith("Digit")) {
        return event.code.slice(5);
    }

    return event.key.toLowerCase();
}

/// Return whether a shortcut originated inside an editable control.
function isEditable(target: EventTarget | null) {
    return (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        (target instanceof HTMLElement && target.isContentEditable)
    );
}
