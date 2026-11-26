import dotsThree from "@destack/icon/phosphor/dots-three";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import {
    createEffect,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import { Icon } from "../icon/index.ts";
import { type MenuContentProperties, useMenu } from "../menu/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { type Axis, ORIENTATION } from "../position/index.ts";
import {
    OverflowContext,
    OverflowControl,
    OverflowEntry,
    OverflowEntryContext,
    useOverflow,
} from "./control.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The styles of an overflow. */
const styles = style.create({
    // take the room there is, asking the elements around for none
    overflow: {
        display: "grid",
        width: "100%",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
        minHeight: 0,
        whiteSpace: "nowrap",
    },
    horizontal: {
        // paint nothing past the row in the frame before it measures
        overflowX: "clip",
        overflowClipMargin: space[1],
        gridTemplateColumns: "minmax(0, 1fr)",
    },
    vertical: {
        gridTemplateRows: "minmax(0, 1fr)",
        width: "auto",
        height: "100%",
    },
    content: {
        position: "relative",
        display: "flex",
        alignItems: "inherit",
        justifyContent: "inherit",
        gap: "inherit",
        minWidth: 0,
        minHeight: 0,
    },
    column: {
        flexDirection: "column",
        height: "100%",
    },
    trigger: {
        flexShrink: 0,
    },
});

/** The properties people change in an overflow's controls and inspector. */
const OVERFLOW_PROPERTIES = {
    orientation: ORIENTATION.described(t`Whether its items line up across or down.`).default(
        "horizontal",
    ),
};

/** The properties of an overflow besides its declared ones: its native attributes. */
export type OverflowAttributes = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style"> &
    ElementPartProperties;

/** Render items along an axis, hiding those that do not fit. */
export const Overflow = defineComponent({
    name: "overflow",
    properties: OVERFLOW_PROPERTIES,
    render(
        properties: RenderProperties<OverflowAttributes, typeof OVERFLOW_PROPERTIES>,
    ): JSX.Element {
        // hide by the room of the element
        const orientation = (): Axis => properties.orientation;
        const control = new OverflowControl(orientation, useEnvironment());

        return (
            <OverflowContext value={control}>
                <OverflowEntryContext value={null}>
                    {renderPart(
                        "div",
                        "overflow",
                        omit(properties, "orientation", "children"),
                        () => [styles.overflow, styles[orientation()]],
                        {
                            get "data-orientation"() {
                                return orientation();
                            },
                            get children() {
                                return (
                                    <div
                                        ref={(element) => control.attach(element)}
                                        {...style.attributes([
                                            styles.content,
                                            orientation() === "vertical" && styles.column,
                                        ])}
                                    >
                                        {properties.children}
                                    </div>
                                );
                            },
                        },
                    )}
                </OverflowEntryContext>
            </OverflowContext>
        );
    },
});

/** The properties of an overflow, its native attributes included. */
export type OverflowProperties = ComponentProperties<typeof Overflow>;

/** The properties of an overflow's item, the native element's attributes included. */
export type OverflowItemProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style"> &
    ElementPartProperties & {
        /** The order in which it hides, the lowest first, zero by default and never at infinity. */
        readonly priority?: number;
        /** What the menu calls it, its own words by default. */
        readonly label?: string;
    };

/** Render something that hides where it does not fit. */
export function OverflowItem(properties: OverflowItemProperties): JSX.Element {
    // join as one entry with everything in it
    let element: HTMLElement | undefined;
    const entry = useOverflow().add({
        kind: "item",
        element: () => element,
        priority: () => properties.priority,
        label: () => properties.label,
    });

    return (
        <OverflowEntryContext value={entry}>
            {renderPart("div", "overflow-item", omit(properties, "priority", "label"), null, {
                ref: (created) => {
                    element = created;
                },
            })}
        </OverflowEntryContext>
    );
}

/** Hold the menu of what does not fit, shown only where something does not. */
export function OverflowMenu(properties: {
    /** The trigger and the content, the standard ones by default. */
    readonly children?: JSX.Element;
}): JSX.Element {
    return (
        <DropdownMenu>
            {contentOr(properties, () => (
                <>
                    <OverflowMenuTrigger />
                    <OverflowMenuContent />
                </>
            ))}
        </DropdownMenu>
    );
}

/** Render the button that opens the menu, which counts as the item it stands in. */
export function OverflowMenuTrigger(properties: ButtonProperties): JSX.Element {
    // show and hide the item around it, or the button by itself
    const control = useOverflow();
    const menu = useMenu();
    const locale = useLocale();
    const around = useContext(OverflowEntryContext);
    let element: HTMLElement | undefined;
    if (around === null) {
        control.add({ kind: "menu", element: () => element });
    } else {
        around.kind = "menu";
    }

    // close the menu once everything fits
    createEffect(
        () => control.hidden().length,
        (count) => {
            if (count === 0 && untrack(() => menu.isOpen())) {
                menu.close(false);
            }
        },
    );

    return (
        <DropdownMenuTrigger
            render={(attributes) => (
                <Button
                    isIconOnly
                    variant="ghost"

                    hidden={around === null}
                    aria-label={locale.render(t.context("action")`More`)}
                    {...mergeProperties(attributes, omit(properties, "xstyle", "children"))}
                    data-slot="overflow-menu-trigger"
                    xstyle={[styles.trigger, properties.xstyle]}
                >
                    {contentOr(properties, () => (
                        <Icon icon={dotsThree} />
                    ))}
                </Button>
            )}
            ref={(created: HTMLElement) => {
                element = created;
            }}
        />
    );
}

/** The properties of an overflow's menu. */
export type OverflowMenuContentProperties = Omit<MenuContentProperties, "children"> & {
    /** Render the item of an entry that does not fit, the standard one by default. */
    readonly children?: (entry: OverflowEntry) => JSX.Element;
};

/** Render the menu of what does not fit. */
export function OverflowMenuContent(properties: OverflowMenuContentProperties): JSX.Element {
    const control = useOverflow();
    const rest = omit(properties, "children");

    return (
        <DropdownMenuContent data-slot="overflow-menu-content" {...rest}>
            <For each={control.hidden()}>
                {(entry) =>
                    properties.children === undefined ? (
                        <OverflowMenuItem entry={entry} />
                    ) : (
                        properties.children(entry)
                    )
                }
            </For>
        </DropdownMenuContent>
    );
}

/** Render the menu item of an entry, which presses it. */
export function OverflowMenuItem(properties: {
    /** The entry. */
    readonly entry: OverflowEntry;
    /** What it shows, the entry's label by default. */
    readonly children?: JSX.Element;
}): JSX.Element {
    // press it once the menu gave the focus back
    const { display } = useEnvironment();
    const menu = useMenu();
    const entry = untrack(() => properties.entry);
    const press = (): void => void display.frame().then(() => entry.press());

    // read it again each time the menu opens
    const read = <Value,>(value: () => Value): Value => {
        menu.isOpen();

        return value();
    };
    const content = (): JSX.Element => contentOr(properties, () => read(() => entry.label()));
    const isChecked = (): boolean | undefined => read(() => entry.isChecked());
    const isDisabled = (): boolean => read(() => entry.isDisabled());

    // check what toggles
    return (
        <Show
            when={isChecked() !== undefined}
            fallback={
                <DropdownMenuItem disabled={isDisabled()} onSelect={press}>
                    {content()}
                </DropdownMenuItem>
            }
        >
            <DropdownMenuCheckboxItem
                checked={isChecked() === true}
                disabled={isDisabled()}
                onSelect={press}
            >
                {content()}
            </DropdownMenuCheckboxItem>
        </Show>
    );
}
