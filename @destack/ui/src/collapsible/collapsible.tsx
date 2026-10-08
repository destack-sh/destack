import * as style from "@destack/style";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    type JSX,
    omit,
    useContext,
} from "@destack/view";
import { followToggle, refuseDisabled } from "../disclosure/index.ts";

/** The nearest collapsible, null outside one. */
const CollapsibleContext = createContext<CollapsibleControl | null>(null);

/** The open state and availability of a collapsible, which its trigger and content read. */
interface CollapsibleControl {
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
export type CollapsibleElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a collapsible, the native disclosure's attributes included. */
export type CollapsibleProperties = CollapsibleElementProperties<
    Omit<JSX.DetailsHtmlAttributes<HTMLDetailsElement>, "open" | "onToggle">
> & {
    /** Whether the content shows, which makes the state controlled. */
    readonly open?: boolean | undefined;
    /** Whether the content shows at first while uncontrolled. */
    readonly defaultOpen?: boolean;
    /** Handle the person showing or hiding the content. */
    readonly onOpenChange?: (open: boolean) => void;
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
    const [isOpen, setOpen] = createControllableSignal({
        isControlled: () => properties.open !== undefined,
        value: () => properties.open === true,
        defaultValue: properties.defaultOpen === true,
        onChange: (open) => properties.onOpenChange?.(open),
    });

    return (
        <CollapsibleContext value={{ isOpen, isDisabled: () => properties.disabled === true }}>
            <details
                data-slot="collapsible"
                data-state={isOpen() ? "open" : "closed"}
                data-disabled={properties.disabled === true ? "" : undefined}
                open={isOpen()}
                {...rest}
                onToggle={(event) =>
                    followToggle(event, isOpen, setOpen, properties.open !== undefined)
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
    const control = useContext(CollapsibleContext);
    const isDisabled = (): boolean => control?.isDisabled() === true;
    const rest = omit(properties, "xstyle", "style");

    return (
        <summary
            data-slot="collapsible-trigger"
            data-state={control?.isOpen() === true ? "open" : "closed"}
            data-disabled={isDisabled() ? "" : undefined}
            aria-disabled={isDisabled() ? "true" : undefined}
            {...rest}
            ref={(element) => refuseDisabled(element, isDisabled)}
            {...style.attributes([styles.trigger, properties.xstyle], properties.style)}
        />
    );
}

/** Render the content a collapsible shows when open. */
export function CollapsibleContent(
    properties: CollapsibleElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useContext(CollapsibleContext);
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="collapsible-content"
            data-state={control?.isOpen() === true ? "open" : "closed"}
            {...rest}
            {...style.attributes([properties.xstyle], properties.style)}
        />
    );
}
