import type { Action, ActionInputs, Choice } from "@destack/object";
import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import {
    createMemo,
    createSignal,
    For,
    type JSX,
    omit,
    Show,
    useAction,
    useActions,
    useLocale,
} from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { mergeProperties } from "../part/index.ts";
import { ChoiceIcon } from "./choice-icon.tsx";

/** The styles of an action's select. */
const styles = style.create({
    empty: {
        color: color.mutedForeground,
    },
    text: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        whiteSpace: "nowrap",
    },
});

/** The properties of an action's select, the native button's attributes included. */
export interface ActionSelectProperties<Inputs extends ActionInputs = ActionInputs> extends Omit<
    ButtonProperties,
    "children"
> {
    /** The action, which asks for one input. */
    readonly action: Action<Inputs>;
    /** What it shows while it has no value, the action's title by default. */
    readonly placeholder?: string;
    /** Render the values it has, each with its icon and title by default. */
    readonly children?: (chosen: readonly Choice[]) => JSX.Element;
}

/** Render a button that shows the value an action sets, and picks another beside it. */
export function ActionSelect<Inputs extends ActionInputs>(
    properties: ActionSelectProperties<Inputs>,
): JSX.Element {
    // read the input it asks for, and the choices where the button stands
    const actions = useActions();
    const locale = useLocale();
    const rest = omit(properties, "action", "placeholder", "children");
    const [button, setButton] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const own = useAction(() => {
        const element = button();

        return element === undefined ? undefined : { action: properties.action, element };
    });
    const chosen = createMemo(() => own.choices().filter((choice) => own.isChosen(choice)));

    // show what it is set to, else what stands for nothing
    const isSet = () => chosen().some((choice) => choice.value !== null);
    const nothing = () =>
        properties.placeholder ?? chosen()[0]?.title ?? locale.render(properties.action.title);
    const label = () =>
        [
            locale.render(properties.action.title),
            locale.list(chosen().map((choice) => choice.title)),
        ]
            .filter((each) => each !== "")
            .join(": ");
    const standard = () => (
        <For each={chosen()}>
            {(choice) => (
                <>
                    <ChoiceIcon choice={choice} />
                    <span {...style.attrs(styles.text)}>{choice.title}</span>
                </>
            )}
        </For>
    );

    return (
        <Button
            variant="ghost"
            size="sm"
            data-slot="action-select"
            data-action={properties.action.name}
            aria-haspopup="dialog"
            aria-label={label()}
            {...mergeProperties(
                {
                    ref: (element: HTMLElement) => setButton(element),
                    onClick: (event: MouseEvent & { readonly currentTarget: HTMLElement }) =>
                        actions.start(properties.action, event.currentTarget),
                },
                rest,
            )}
        >
            <Show
                when={isSet() || (chosen().length > 0 && properties.children !== undefined)}
                fallback={<span {...style.attrs(styles.text, styles.empty)}>{nothing()}</span>}
            >
                {properties.children === undefined ? standard() : properties.children(chosen())}
            </Show>
        </Button>
    );
}
