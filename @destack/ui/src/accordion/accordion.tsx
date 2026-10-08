import { Icon } from "@destack/icon";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createUniqueId,
    type JSX,
    omit,
    useContext,
    useLocale,
} from "@destack/view";
import { Selection, type SelectionProperties } from "../selection/index.ts";
import { followToggle, refuseDisabled } from "../disclosure/index.ts";
import { ListState } from "../focus/index.ts";

/** The open items of the nearest accordion, null outside one. */
const AccordionContext = createContext<AccordionControl | null>(null);

/** The item of the nearest accordion item, null outside one. */
const AccordionItemContext = createContext<AccordionItemControl | null>(null);

/** The styles of an accordion and its items. */
const styles = style.create({
    item: {
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        borderBottomColor: color.border,
        interpolateSize: "allow-keywords",
        "::details-content": {
            height: { default: 0, ":open": "auto" },
            overflow: "clip",
            transitionProperty: "height, content-visibility",
            transitionDuration: motion.durationMedium,
            transitionTimingFunction: motion.easingStandard,
            transitionBehavior: "allow-discrete",
        },
    },
    trigger: {
        display: "flex",
        alignItems: "flex-start",
        justifyContent: "space-between",
        gap: space[4],
        paddingBlock: space[4],
        borderRadius: radius[3],
        fontWeight: weight.medium,
        listStyle: "none",
        cursor: { default: "pointer", ":is([aria-disabled=true])": "not-allowed" },
        opacity: { default: 1, ":is([aria-disabled=true])": 0.5 },
        textDecoration: {
            default: "none",
            ":hover": { default: null, [media.hover]: "underline" },
        },
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        "::-webkit-details-marker": { display: "none" },
    },
    chevron: {
        display: "inline-flex",
        flexShrink: 0,
        color: color.mutedForeground,
        transform: { default: "none", [style.when.ancestor(":open")]: "rotate(180deg)" },
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    content: {
        paddingBottom: space[4],
    },
});

/** The properties of an accordion, the native element's attributes included. */
export type AccordionProperties = AccordionLook & SelectionProperties;

/** The layout and availability of an accordion, the native element's attributes included. */
export interface AccordionLook extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onKeyDown"
> {
    /** The direction the arrow keys move between triggers along, vertical by default. */
    readonly orientation?: "horizontal" | "vertical";
    /** Whether every trigger ignores the person. */
    readonly disabled?: boolean;
    /** The StyleX styles applied after the accordion's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of an item of an accordion, the native disclosure's attributes included. */
export type AccordionItemProperties = AccordionElementProperties<
    Omit<JSX.DetailsHtmlAttributes<HTMLDetailsElement>, "open" | "name" | "onToggle">
> & {
    /** The value the item stands for in the accordion's open values. */
    readonly value: string;
    /** Whether the item's trigger ignores the person. */
    readonly disabled?: boolean;
};

/** The properties of an element of an accordion, the native element's attributes included. */
export type AccordionElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The open items of an accordion and its triggers, which its items share. */
export class AccordionControl {
    /** The properties of the accordion root, read for its controlled state. */
    readonly properties: AccordionProperties;
    /** The name a single accordion's items share so the platform keeps one open, undefined for a multiple one. */
    readonly name: string | undefined;
    /** The open values, controlled or the accordion's own. */
    readonly selection: Selection;
    /** The triggers in document order, which the arrow keys move between. */
    readonly list: ListState;

    /** Create the state of an accordion from its root's properties. */
    constructor(properties: AccordionProperties) {
        // follow the controlled open values or the accordion's own
        this.properties = properties;
        this.name = properties.multiple === true ? undefined : createUniqueId();
        this.selection = new Selection(properties);
        this.list = new ListState({
            get orientation() {
                return properties.orientation ?? "vertical";
            },
            isLooping: true,
            isTypeahead: false,
        });
    }

    /** Report whether an item is open. */
    isOpen(value: string): boolean {
        return this.selection.isSelected(value);
    }

    /** Open or close an item as its disclosure toggled, keeping another item a single accordion opened in its place. */
    toggle(value: string, isOpening: boolean, element: HTMLDetailsElement): void {
        // leave the state alone when a single accordion closes an item for another
        const isReplaced =
            this.name !== undefined &&
            element.parentElement?.querySelector(`details[name="${this.name}"][open]`) != null;
        if (!isOpening && isReplaced) {
            return;
        }

        // open or close the item
        if (isOpening !== this.isOpen(value)) {
            this.selection.toggle(value);
        }
    }
}

/** The value and availability of an accordion item, which its trigger reads. */
interface AccordionItemControl {
    /** The value the item stands for. */
    readonly value: string;
    /** Whether the item's trigger ignores the person. */
    readonly isDisabled: Accessor<boolean>;
    /** Whether the item is open. */
    readonly isOpen: Accessor<boolean>;
}

/** Render a stack of disclosures, of which a single accordion keeps one open, that arrow keys move between. */
export function Accordion(properties: AccordionProperties): JSX.Element {
    // share the open items with the items and read the text direction for the arrow keys
    const control = new AccordionControl(properties);
    const locale = useLocale();
    const rest = omit(
        properties,
        "multiple",
        "value",
        "defaultValue",
        "onValueChange",
        "orientation",
        "disabled",
        "xstyle",
        "style",
    );

    return (
        <AccordionContext value={control}>
            <div
                data-slot="accordion"
                data-orientation={properties.orientation ?? "vertical"}
                {...rest}
                onKeyDown={(event) => {
                    // move between the triggers when the focus rests on one
                    if (
                        event.target instanceof HTMLElement &&
                        event.target.dataset["slot"] === "accordion-trigger"
                    ) {
                        control.list.focus.move(event, locale.direction);
                    }
                }}
                onFocusOut={(event) => control.list.focus.focusOut(event)}
                {...style.attributes([properties.xstyle], properties.style)}
            />
        </AccordionContext>
    );
}

/** Render one native disclosure of the nearest accordion, open while its value is. */
export function AccordionItem(properties: AccordionItemProperties): JSX.Element {
    // read the accordion, refusing an item outside one
    const accordion = useContext(AccordionContext);
    if (accordion === null) {
        throw new TypeError("an accordion item needs an accordion around it");
    }
    const rest = omit(properties, "value", "disabled", "xstyle", "style");
    const isDisabled = (): boolean =>
        properties.disabled === true || accordion.properties.disabled === true;

    return (
        <AccordionItemContext
            value={{
                value: properties.value,
                isDisabled,
                isOpen: () => accordion.isOpen(properties.value),
            }}
        >
            <details
                name={accordion.name}
                data-slot="accordion-item"
                data-state={accordion.isOpen(properties.value) ? "open" : "closed"}
                data-disabled={isDisabled() ? "" : undefined}
                open={accordion.isOpen(properties.value)}
                {...rest}
                onToggle={(event) =>
                    followToggle(
                        event,
                        () => accordion.isOpen(properties.value),
                        (isOpening) =>
                            accordion.toggle(properties.value, isOpening, event.currentTarget),
                        "value" in accordion.properties,
                    )
                }
                {...style.attributes(
                    [style.defaultMarker(), styles.item, properties.xstyle],
                    properties.style,
                )}
            />
        </AccordionItemContext>
    );
}

/** Render the summary that toggles its item, with a chevron that turns when open. */
export function AccordionTrigger(
    properties: AccordionElementProperties<
        Omit<JSX.HTMLAttributes<HTMLElement>, "ref" | "onFocus">
    >,
): JSX.Element {
    // read the item, refusing a trigger outside one, and join the accordion's triggers
    const accordion = useContext(AccordionContext);
    const item = useContext(AccordionItemContext);
    if (accordion === null || item === null) {
        throw new TypeError("an accordion trigger needs an accordion item around it");
    }
    const isDisabled = (): boolean => item.isDisabled();
    const rest = omit(properties, "xstyle", "style", "children");
    let element: HTMLElement | undefined;
    accordion.list.add({
        key: item.value,
        text: () => item.value,
        isDisabled,
        element: () => element,
    });

    return (
        <summary
            data-slot="accordion-trigger"
            data-state={item.isOpen() ? "open" : "closed"}
            data-disabled={isDisabled() ? "" : undefined}
            aria-disabled={isDisabled() ? "true" : undefined}
            {...rest}
            ref={(summary) => {
                element = summary;
                refuseDisabled(summary, isDisabled);
            }}
            onFocus={() => accordion.list.focus.focusIn(item.value)}
            {...style.attributes(
                [text.callout, styles.trigger, properties.xstyle],
                properties.style,
            )}
        >
            {properties.children}
            <span {...style.attrs(styles.chevron)}>
                <Icon name="caret-down" />
            </span>
        </summary>
    );
}
/** Render the content an item reveals when open. */
export function AccordionContent(
    properties: AccordionElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const item = useContext(AccordionItemContext);
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="accordion-content"
            data-state={item?.isOpen() === true ? "open" : "closed"}
            {...rest}
            {...style.attributes(
                [text.footnote, styles.content, properties.xstyle],
                properties.style,
            )}
        />
    );
}
