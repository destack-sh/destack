import caretLeftIcon from "@destack/icon/phosphor/caret-left";
import caretRightIcon from "@destack/icon/phosphor/caret-right";
import * as style from "@destack/style";
import { color, motion, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createUniqueId,
    type JSX,
    omit,
    useContext,
    useLocale,
} from "@destack/view";
import { disclosure, DisclosurePanel, followToggle, refuseDisabled } from "../disclosure/index.ts";
import { Icon } from "../icon/index.ts";
import { OpenChange, type OpenProperties, OpenState } from "../layer/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The nearest collapsible, null outside one. */
const CollapsibleContext = createContext<CollapsibleControl | null>(null);

/** Read the nearest collapsible, refusing a part outside one. */
export function useCollapsible(): CollapsibleControl {
    const control = useContext(CollapsibleContext);
    if (control === null) {
        throw new TypeError("a collapsible part needs a collapsible around it");
    }

    return control;
}

/** The open state and availability of a collapsible, which its trigger and content read. */
export interface CollapsibleControl {
    /** Whether the content shows. */
    readonly isOpen: Accessor<boolean>;
    /** Whether the trigger ignores the person. */
    readonly isDisabled: Accessor<boolean>;
}

/** The styles of a collapsible and its parts. */
const styles = style.create({
    trigger: {
        listStyle: "none",
        cursor: { default: "pointer", ":is([aria-disabled=true])": "not-allowed" },
        "::-webkit-details-marker": { display: "none" },
    },
    indicator: {
        display: "inline-flex",
        flexShrink: 0,
        transitionProperty: "rotate",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    turned: { rotate: "90deg" },
    turnedBack: { rotate: "-90deg" },
    // run a line down the start of it, below the middle of the trigger's icon
    line: {
        minWidth: 0,
        marginBlockStart: space[2],
        marginInlineStart: `calc((1em - ${stroke.border}) / 2)`,
        paddingInlineStart: space[4],
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
    },
});

/** The properties of an element of a collapsible, the native element's attributes included. */
export type CollapsibleElementProperties<Attributes> = Omit<Attributes, "class"> &
    ElementPartProperties;

/** The properties of a collapsible, the native disclosure's attributes included. */
export type CollapsibleProperties = CollapsibleElementProperties<
    Omit<JSX.DetailsHtmlAttributes<HTMLDetailsElement>, "open" | "onToggle">
> &
    OpenProperties & {
        /** Whether the trigger ignores the person. */
        readonly disabled?: boolean;
    };

/** Render a native disclosure that shows and hides its content. */
export function Collapsible(properties: CollapsibleProperties): JSX.Element {
    // follow the controlled state or the collapsible's own
    const rest = omit(
        properties,
        "open",
        "defaultOpen",
        "onOpenChange",
        "disabled",
        "xstyle",
        "style",
    );
    const collapsible = new OpenState(properties, createUniqueId());
    const { isOpen } = collapsible;

    return (
        <CollapsibleContext value={{ isOpen, isDisabled: () => properties.disabled === true }}>
            <details
                data-slot="collapsible"
                data-open={isOpen() ? "" : undefined}
                data-closed={isOpen() ? undefined : ""}
                data-disabled={properties.disabled === true ? "" : undefined}
                open={isOpen()}
                {...rest}
                onToggle={(event) =>
                    followToggle(
                        event,
                        isOpen,
                        (open) => collapsible.follow(open, new OpenChange("trigger-press", event)),
                        true,
                    )
                }
                {...style.attributes([disclosure.root, properties.xstyle], properties.style)}
            />
        </CollapsibleContext>
    );
}

/** Render the summary that toggles its collapsible. */
export function CollapsibleTrigger(
    properties: CollapsibleElementProperties<Omit<JSX.HTMLAttributes<HTMLElement>, "ref">>,
): JSX.Element {
    // read the collapsible's state and availability
    const control = useCollapsible();
    const isDisabled = (): boolean => control.isDisabled();

    return renderPart("summary", "collapsible-trigger", properties, styles.trigger, {
        get "data-panel-open"() {
            return control.isOpen() ? "" : undefined;
        },
        get "data-disabled"() {
            return isDisabled() ? "" : undefined;
        },
        get "aria-disabled"() {
            return isDisabled() ? "true" : undefined;
        },
        ref: (element) => refuseDisabled(element, isDisabled),
    });
}

/** Render the mark of a trigger that turns as its collapsible opens, a caret by default. */
export function CollapsibleIndicator(
    properties: CollapsibleElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    // turn down from the reading direction
    const control = useCollapsible();
    const locale = useLocale();
    const isRightToLeft = (): boolean => locale.direction === "rtl";
    const rest = omit(properties, "children");

    return renderPart(
        "span",
        "collapsible-indicator",
        rest,
        () => [
            styles.indicator,
            control.isOpen() && (isRightToLeft() ? styles.turnedBack : styles.turned),
        ],
        {
            "aria-hidden": "true" as const,
            get children() {
                return (
                    properties.children ?? (
                        <Icon icon={isRightToLeft() ? caretLeftIcon : caretRightIcon} />
                    )
                );
            },
        },
    );
}

/** The properties of a collapsible's content, the native element's attributes included. */
export type CollapsibleContentProperties = CollapsibleElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> & {
    /** The layout, plain by default, or along a line below the icon of its trigger. */
    readonly variant?: "default" | "line";
};

/** Render the content a collapsible shows when open. */
export function CollapsibleContent(properties: CollapsibleContentProperties): JSX.Element {
    // lay it out along a line when asked
    const control = useCollapsible();
    const rest = omit(properties, "variant");
    const layout = (): style.Styles => (properties.variant === "line" ? styles.line : null);

    return (
        <DisclosurePanel isOpen={() => control.isOpen()}>
            {renderPart("div", "collapsible-content", rest, layout, {
                get "data-open"() {
                    return control.isOpen() ? "" : undefined;
                },
                get "data-closed"() {
                    return control.isOpen() ? undefined : "";
                },
            })}
        </DisclosurePanel>
    );
}
