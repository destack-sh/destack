import type { ObjectType } from "@destack/object";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { Badge } from "@destack/ui/badge";
import type { ButtonVariant } from "@destack/ui/button";
import type { ScopeCommands } from "../scope/scope.ts";
import type { JSX } from "../solid/component.ts";
import { For, Loading } from "../solid/flow.ts";
import { createMemo, omit } from "../solid/reactive.ts";
import { CommandButton } from "./command-button.tsx";

/** The styles of a state transition control. */
const styles = style.create({
    group: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
    },
});

/** A transition of a state field: the method that makes it, the state it enters and the permission it needs. */
export interface StateTransitionOption {
    /** The method making the transition. */
    readonly name: string;
    /** The state it enters. */
    readonly to: string;
    /** The permission the caller needs. */
    readonly permission: string;
}

/** The properties of a state transition control, the native element's attributes included. */
export interface StateTransitionProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
    /** The scope the object lives in, which checks the person's permissions and calls the transition's method. */
    readonly access: ScopeCommands;
    /** The object type declaring the state field. */
    readonly object: ObjectType;
    /** The object's identifier. */
    readonly id: string;
    /** The state field's name. */
    readonly field: string;
    /** The object's state now. */
    readonly value: string;
    /** The text of each state and of each transition's button, their names by default. */
    readonly labels?: Readonly<Record<string, string>>;
    /** The look of each button, outline by default. */
    readonly variant?: ButtonVariant;
    /** The StyleX styles applied after the group's styles. */
    readonly style?: style.Styles;
}

/** List the transitions a state field's machine allows from a state, refusing a field without a machine. */
export function transitionsOf(
    object: ObjectType,
    field: string,
    state: string,
): StateTransitionOption[] {
    const machine = object.fields[field]?.machine;
    if (machine === undefined) {
        throw new TypeError(`${object.name}.${field} is no state field`);
    }

    return Object.entries(machine.transitions)
        .filter(([, transition]) => transition.from.includes(state))
        .map(([name, transition]) => ({
            name,
            to: transition.to,
            permission: transition.permission,
        }));
}

/** Render an object's state with a button for each transition its machine allows from it and the person may make. */
export function StateTransition(properties: StateTransitionProperties): JSX.Element {
    // keep the transitions whose permission the person holds on the object
    const rest = omit(
        properties,
        "access",
        "object",
        "id",
        "field",
        "value",
        "labels",
        "variant",
        "style",
    );
    const offered = createMemo(async () => {
        const transitions = transitionsOf(properties.object, properties.field, properties.value);
        const allowed = await Promise.all(
            transitions.map((transition) =>
                properties.access.can(transition.permission, {
                    type: properties.object,
                    id: properties.id,
                }),
            ),
        );

        return transitions.filter((_, index) => allowed[index] === true);
    });
    const label = (name: string): string => properties.labels?.[name] ?? name;

    return (
        <div
            role="group"
            data-slot="state-transition"
            data-state={properties.value}
            {...rest}
            {...style.attrs(styles.group, properties.style)}
        >
            <Badge variant="secondary" data-slot="state-transition-state">
                {label(properties.value)}
            </Badge>
            <Loading>
                <For each={offered()}>
                    {(transition) => (
                        <CommandButton
                            variant={properties.variant ?? "outline"}
                            size="sm"
                            data-transition={transition.name}
                            run={() =>
                                properties.access.call(properties.object, transition.name, {
                                    id: properties.id,
                                })
                            }
                        >
                            {label(transition.name)}
                        </CommandButton>
                    )}
                </For>
            </Loading>
        </div>
    );
}
