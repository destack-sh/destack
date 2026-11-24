import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type Action, type ActionInputs, type ValuesOf } from "@destack/object";
import { Icon } from "@destack/ui/icon";
import type { IconBodies } from "@destack/icon";
import { Shortcut } from "@destack/locale";
import {
    createMemo,
    createSignal,
    type JSX,
    omit,
    onSettled,
    Show,
    useActions,
    useLocale,
} from "@destack/view";
import { mergeProperties } from "../part/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import { useCommandPlatform } from "../kbd/index.ts";
import { Tooltip, TooltipContent, TooltipTrigger } from "../tooltip/index.ts";

/** The style of the keys in a button's tooltip. */
const styles = style.create({
    keys: {
        marginInlineStart: space[2],
        opacity: 0.7,
    },
});

/** The properties of a button that runs an action. */
export interface ActionButtonProperties<Inputs extends ActionInputs = ActionInputs> extends Omit<
    ButtonProperties,
    "onClick"
> {
    /** The action. */
    readonly action: Action<Inputs>;
    /** The values the button gives, the rest from where the person is. */
    readonly given?: Partial<ValuesOf<Inputs>>;
    /** Where it runs, where the button stands or the person last was by default. */
    readonly element?: Element | null;
    /** The icon, beside the title or alone in an icon size. */
    readonly icon?: IconBodies;
}

/** Render a button that runs an action where it stands, else where the person last was. */
export function ActionButton<Inputs extends ActionInputs>(
    properties: ActionButtonProperties<Inputs>,
): JSX.Element {
    // read the action's title and the keys that run it
    const actions = useActions();
    const locale = useLocale();
    const isCommand = useCommandPlatform();
    const rest = omit(properties, "action", "given", "element", "icon", "variant", "children");
    const title = () => locale.render(properties.action.title);
    const keys = () => actions.keybinding(properties.action);
    const isAlone = () => properties.size === "icon" || properties.size === "icon-sm";
    const [button, setButton] = createSignal<Element | undefined>(undefined, { ownedWrite: true });
    const place = () => {
        // find where a press would run it
        const own = button();

        return properties.element !== undefined || own === undefined
            ? (properties.element ?? null)
            : actions.placeOf(properties.action, own);
    };
    let created: HTMLElement | undefined;
    onSettled(() => {
        // look for what runs it once it is in the page
        setButton(created);
    });
    const isOn = createMemo(() => actions.isOn(properties.action, place()));
    const isDisabled = createMemo(
        () =>
            properties.disabled === true ||
            (button() !== undefined && !actions.has(properties.action.reference, place())),
    );
    const pressed = () => {
        const on = isOn();

        return on === undefined ? undefined : on ? "true" : "false";
    };
    const run = () => {
        // leave an action that does not run, whose button stays in the tab order
        if (!isDisabled()) {
            actions.start(properties.action, place(), properties.given);
        }
    };

    return (
        <Tooltip>
            <TooltipTrigger
                render={(attributes) => (
                    <Button
                        {...mergeProperties(attributes, {
                            ref: (element: HTMLElement) => (created = element),
                        })}
                        {...rest}
                        data-slot="action-button"
                        data-action={properties.action.name}
                        aria-label={isAlone() ? title() : undefined}
                        aria-pressed={pressed()}
                        variant={isOn() === true ? "secondary" : (properties.variant ?? "default")}
                        aria-keyshortcuts={
                            keys() === undefined
                                ? undefined
                                : Shortcut.keys(keys() ?? "", isCommand()).join("+")
                        }
                        aria-disabled={isDisabled() ? "true" : undefined}
                        onClick={run}
                    />
                )}
            >
                <Show when={properties.icon}>{(icon) => <Icon icon={icon()} />}</Show>
                <Show when={!isAlone()}>{properties.children ?? title()}</Show>
            </TooltipTrigger>
            <TooltipContent>
                {title()}
                <Show when={keys()}>
                    {(shortcut) => (
                        <span {...style.attrs(styles.keys)}>
                            {Shortcut.format(shortcut(), isCommand())}
                        </span>
                    )}
                </Show>
            </TooltipContent>
        </Tooltip>
    );
}
