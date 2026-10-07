import { type Accessor, createSignal } from "@solidjs/signals";

/** A value its holder keeps unless its owner controls it by passing it, after controlled and uncontrolled inputs. */
export interface ControllableValue<Value> {
    /** Whether the owner passes the value, which makes it controlled. */
    readonly isControlled: () => boolean;
    /** The value the owner passes. */
    readonly value: () => Value;
    /** The value the holder starts from while uncontrolled. */
    readonly defaultValue: Value;
    /** Tell the owner of a change. */
    readonly onChange?: (value: Value) => void;
}

/** Follow the value the owner controls, else the holder's own, telling the owner of each change. */
export function createControllableSignal<Value>(
    controllable: ControllableValue<Value>,
): [Accessor<Value>, (value: Value) => void] {
    const [own, setOwn] = createSignal<{ readonly value: Value }>({
        value: controllable.defaultValue,
    });

    return [
        () => (controllable.isControlled() ? controllable.value() : own().value),
        (value) => {
            // keep the change while uncontrolled and tell the owner either way
            setOwn({ value });
            controllable.onChange?.(value);
        },
    ];
}
