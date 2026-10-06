import { type Accessor, createControllableSignal } from "@destack/view";

/** The checked state of a native checkbox, controlled by passing `checked`. */
export interface CheckProperties {
    /** Whether the box is checked, which makes the state controlled. */
    readonly checked?: boolean | undefined;
    /** Whether the box starts checked while uncontrolled. */
    readonly defaultChecked?: boolean | undefined;
    /** Handle the person checking or unchecking the box. */
    readonly onCheckedChange?: ((checked: boolean) => void) | undefined;
}

/** The checked state of a native checkbox and the change handler that keeps the element in step with it. */
export interface CheckControl {
    /** Whether the box is checked. */
    readonly isChecked: Accessor<boolean>;
    /** Take the person's change, holding the element at a controlled state its owner keeps. */
    readonly onChange: (event: Event & { readonly currentTarget: HTMLInputElement }) => void;
}

/** Follow a native checkbox's checked state, controlled or its own. */
export function useCheckControl(properties: CheckProperties): CheckControl {
    const [isChecked, setChecked] = createControllableSignal({
        isControlled: () => properties.checked !== undefined,
        value: () => properties.checked === true,
        defaultValue: properties.defaultChecked === true,
        onChange: (checked) => properties.onCheckedChange?.(checked),
    });

    return {
        isChecked,
        onChange: (event) => {
            // keep the change and hold a controlled box at its owner's state
            setChecked(event.currentTarget.checked);
            if (properties.checked !== undefined) {
                event.currentTarget.checked = properties.checked;
            }
        },
    };
}
