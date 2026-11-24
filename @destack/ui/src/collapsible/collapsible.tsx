import * as style from "@destack/style";
import {
    type Accessor,
    createContext,
    createUniqueId,
    type JSX,
    omit,
    useContext,
} from "@destack/view";
import { followToggle, refuseDisabled } from "../disclosure/index.ts";
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

/** The styles of a collapsible's trigger. */
const styles = style.create({
    trigger: {
        listStyle: "none",
        cursor: { default: "pointer", ":is([aria-disabled=true])": "not-allowed" },
        "::-webkit-details-marker": { display: "none" },
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
                {...style.attributes([properties.xstyle], properties.style)}
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

/** Render the content a collapsible shows when open. */
export function CollapsibleContent(
    properties: CollapsibleElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useCollapsible();

    return renderPart("div", "collapsible-content", properties, null, {
        get "data-open"() {
            return control.isOpen() ? "" : undefined;
        },
        get "data-closed"() {
            return control.isOpen() ? undefined : "";
        },
    });
}
