import xIcon from "@destack/icon/phosphor/x";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, motion, radius, shadow, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { createContext, type JSX, omit, Show, useContext, useLocale } from "@destack/view";
import { clearSelection } from "./action.ts";
import { type ItemPlace, provideItems } from "./place.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "../dropdown-menu/index.ts";
import { Icon } from "../icon/index.ts";
import { MenuActionList } from "./action-menu.tsx";
import { contentOr, type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of an action bar and its parts. */
const styles = style.create({
    bar: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    inline: {
        paddingBlock: space[2],
    },
    floating: {
        position: "absolute",
        insetBlockEnd: space[5],
        insetInlineStart: "50%",
        zIndex: 2,
        maxWidth: `calc(100% - 2 * ${space[4]})`,
        padding: space[1],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.popover,
        boxShadow: shadow.overlay,
        transform: {
            default: "translateX(-50%)",
            ":dir(rtl)": "translateX(50%)",
        },
        opacity: { default: 1, "@starting-style": 0 },
        translate: { default: "none", "@starting-style": `0 ${space[2]}` },
        transitionProperty: "opacity, translate",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingEmphasised,
    },
    count: {
        paddingInline: space[2],
        fontWeight: weight.medium,
        fontVariantNumeric: "tabular-nums",
        whiteSpace: "nowrap",
    },
    clear: {
        flexShrink: 0,
    },
});

/** What an action bar's parts read. */
interface ActionBarControl {
    /** Read how many items are selected. */
    readonly count: () => number;
    /** Read what clears the selection, absent where nothing does. */
    readonly clear: () => (() => void) | undefined;
}

/** The action bar of the nearest action bar, null outside one. */
const ActionBarContext = createContext<ActionBarControl | null>(null);

/** Where an action bar stands: over the end of what is around it, or in the flow. */
export type ActionBarPlacement = "floating" | "inline";

/** The properties of an action bar, the native element's attributes included. */
export interface ActionBarProperties
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">, ElementPartProperties {
    /** The selected items, the bar showing while any are, which the actions in it act on. */
    readonly items: readonly string[];
    /** Read the object an item stands for, which actions on its type take, absent for none. */
    readonly reference?: ItemPlace["reference"];
    /** Clear the selection. */
    readonly onClear?: () => void;
    /** Where it stands, floating by default. */
    readonly placement?: ActionBarPlacement;
}

/** The properties of an element of an action bar, the native element's attributes included. */
export type ActionBarElementProperties = Omit<
    JSX.HTMLAttributes<HTMLSpanElement>,
    "class" | "style"
> &
    ElementPartProperties;

/** Render the actions on selected items while any are, after React Spectrum's action bar. */
export function ActionBar(properties: ActionBarProperties): JSX.Element {
    // show while something is selected
    const locale = useLocale();
    const rest = omit(properties, "items", "reference", "onClear", "placement", "children");
    const placement = (): ActionBarPlacement => properties.placement ?? "floating";

    return (
        <ActionBarContext
            value={{ count: () => properties.items.length, clear: () => properties.onClear }}
        >
            <Show when={properties.items.length > 0}>
                {renderPart(
                    "div",
                    "action-bar",
                    rest,
                    () => [text.sm, styles.bar, styles[placement()]],
                    {
                        role: "group" as const,
                        ref: provideItems({
                            targets: () => properties.items,
                            reference: (key) => properties.reference?.(key),
                        }),
                        get "aria-label"() {
                            return locale.render(t`Actions`);
                        },
                        get "data-placement"() {
                            return placement();
                        },
                        get children() {
                            return contentOr(properties, () => (
                                <>
                                    <ActionBarCount />
                                    <ActionBarMenu />
                                    <ActionBarClear />
                                </>
                            ));
                        },
                    },
                )}
            </Show>
        </ActionBarContext>
    );
}

/** Render how many items are selected, announced as it changes. */
export function ActionBarCount(properties: ActionBarElementProperties): JSX.Element {
    const locale = useLocale();
    const { count } = useActionBar();

    return renderPart("span", "action-bar-count", properties, styles.count, {
        "aria-live": "polite",
        get children() {
            return contentOr(properties, () =>
                locale.render(t`${locale.number(count())} selected`),
            );
        },
    });
}

/** Render the button whose menu lists every action on the selected items. */
export function ActionBarMenu(properties: ButtonProperties): JSX.Element {
    const locale = useLocale();

    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        size="sm"
                        variant="ghost"
                        data-slot="action-bar-menu"
                        {...properties}
                    />
                )}
            >
                {contentOr(properties, () => locale.render(t`Actions`))}
            </DropdownMenuTrigger>
            <DropdownMenuContent side="top">
                <MenuActionList />
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

/** Render the button that clears the selection. */
export function ActionBarClear(properties: ButtonProperties): JSX.Element {
    const locale = useLocale();
    const { clear } = useActionBar();

    return (
        <Show when={clear()}>
            {(run) => (
                <Button
                    size="icon-sm"
                    variant="ghost"
                    data-slot="action-bar-clear"
                    aria-label={locale.render(clearSelection.title)}
                    onClick={() => run()()}
                    {...properties}
                >
                    {contentOr(properties, () => (
                        <Icon icon={xIcon} />
                    ))}
                </Button>
            )}
        </Show>
    );
}

/** Read the nearest action bar, refusing parts outside one. */
export function useActionBar(): ActionBarControl {
    const control = useContext(ActionBarContext);
    if (control === null) {
        throw new TypeError("an action bar part needs an action bar around it");
    }

    return control;
}
