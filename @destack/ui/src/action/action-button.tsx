import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { type Action, type ActionInputs, type ValuesOf } from "@destack/object";
import { Icon } from "@destack/ui/icon";
import type { IconBodies, IconWeight } from "@destack/icon";
import { Shortcut } from "@destack/locale";
import {
    createSignal,
    type JSX,
    omit,
    onSettled,
    Show,
    useAction,
    useActions,
    useLocale,
} from "@destack/view";
import { contentOr, type PartAttributes } from "../part/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import { useCommandPlatform } from "../kbd/index.ts";

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
    /** The icon, beside the title, or alone where it shows only its icon. */
    readonly icon?: IconBodies;
    /** Whether it shows only its icon, square, its title in its tooltip. */
    readonly isIconOnly?: boolean;
    /** The weight the icon is drawn in, regular by default. */
    readonly weight?: IconWeight;
    /** The address a press follows as a link, doing what the action does. */
    readonly href?: string;
    /** The name a press saves the address under, followed instead by default. */
    readonly download?: string;
}

/** Render a button that runs an action where it stands, else where the person last was. */
export function ActionButton<Inputs extends ActionInputs>(
    properties: ActionButtonProperties<Inputs>,
): JSX.Element {
    // read the action's title and the keys that run it
    const actions = useActions();
    const locale = useLocale();
    const isCommand = useCommandPlatform();
    const rest = omit(
        properties,
        "action",
        "given",
        "element",
        "icon",
        "weight",
        "href",
        "download",
        "variant",
        "children",
        "isIconOnly",
    );
    const title = () => locale.render(properties.action.title);
    const keys = () => actions.keybindings(properties.action, isCommand())[0];
    const isAlone = () => properties.isIconOnly === true;
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
    const own = useAction(() => ({
        action: properties.action,
        element: place(),
        given: properties.given,
        isFollowed: false,
    }));
    const isDisabled = () =>
        properties.disabled === true || (button() !== undefined && !own.isEnabled());
    const pressed = () => {
        const on = own.isOn();

        return on === undefined ? undefined : on ? "true" : "false";
    };
    const run = (event: MouseEvent) => {
        // leave an action that does not run, whose button stays in the tab order
        if (isDisabled()) {
            event.preventDefault();
        }
        // run it, where the browser follows no link
        else if (properties.href === undefined) {
            own.start();
        }
    };

    // keep the page where a browser opens what it does not save, as it does of another origin
    const link = (attributes: PartAttributes): JSX.Element => (
        <a
            {...attributes}
            href={properties.href}
            download={properties.download}
            target={properties.download === undefined ? undefined : "_blank"}
            rel={properties.download === undefined ? undefined : "noreferrer"}
        />
    );

    return (
        <Button
            ref={(element: HTMLElement) => (created = element)}
            data-slot="action-button"
            {...rest}
            {...(properties.href === undefined ? {} : { render: link })}
            data-action={properties.action.name}
            aria-label={properties["aria-label"] ?? (isAlone() ? title() : undefined)}
            isIconOnly={isAlone()}
            aria-pressed={pressed()}
            variant={own.isOn() === true ? "secondary" : (properties.variant ?? "default")}
            aria-keyshortcuts={
                keys() === undefined ? undefined : Shortcut.aria(keys() ?? "", isCommand())
            }
            aria-disabled={isDisabled() ? "true" : undefined}
            onClick={run}
            tooltip={
                <>
                    {title()}
                    <Show when={keys()}>
                        {(shortcut) => (
                            <span {...style.attrs(styles.keys)}>
                                {Shortcut.format(shortcut(), isCommand())}
                            </span>
                        )}
                    </Show>
                </>
            }
        >
            <Show when={properties.icon}>
                {(icon) => <Icon icon={icon()} weight={properties.weight ?? "regular"} />}
            </Show>
            <Show when={!isAlone()}>{contentOr(properties, () => title())}</Show>
        </Button>
    );
}
