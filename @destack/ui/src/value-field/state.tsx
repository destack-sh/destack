import { Icon } from "../icon/index.ts";
import resetIcon from "@destack/icon/phosphor/arrow-counter-clockwise";
import type { IconName } from "@destack/icon/name";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import { type Accessor, createContext, type JSX, Show, useContext, useLocale } from "@destack/view";
import { LazyIcon } from "@destack/ui/icon";
import { Button } from "../button/index.ts";
import { useFieldControl } from "../field/control.ts";
import { FieldLabel } from "../field/field.tsx";
import { Tooltip, TooltipContent, TooltipTrigger } from "../tooltip/index.ts";

/** The styles of a value's label line and its states. */
export const stateStyles = style.create({
    label: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
    },
    unset: {
        color: color.mutedForeground,
    },
    trigger: {
        display: "flex",
        minWidth: 0,
    },
    name: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        unicodeBidi: "plaintext",
    },
    mixed: {
        width: "100%",
        justifyContent: "flex-start",
        color: color.mutedForeground,
    },
});

/** Where a value stands: given, left to the one in force, or held apart by several objects. */
export type ValueState = "set" | "unset" | "mixed";

/** A value's state and its reset, as its field gives them to the legend of its set of parts. */
export interface ValueHeading {
    /** Where the value stands. */
    readonly state: Accessor<ValueState>;
    /** The button that gives the value back to the one in force, shown while one is given. */
    readonly reset: JSX.Element;
}

/** The heading of the nearest value's field, for the set of parts that draws its legend. */
export const ValueHeadingContext = createContext<ValueHeading | null>(null);

/** Read the heading of the nearest value's field, null outside one or once a set took it. */
export function useValueHeading(): ValueHeading | null {
    return useContext(ValueHeadingContext);
}

/** Render a value's label: its icon and name, muted while unset, its description pointed at. */
export function ValueLabel(properties: {
    /** What the value is called. */
    readonly label: string;
    /** The icon beside the name, none for none. */
    readonly icon: IconName | undefined;
    /** One sentence on what the value takes, shown when the label is pointed at. */
    readonly description: string | undefined;
    /** Where the value stands. */
    readonly state: ValueState;
}): JSX.Element {
    // draw the icon and the name, muted while the value is left to the one in force
    const label = (): JSX.Element => (
        <FieldLabel xstyle={[stateStyles.label, properties.state === "unset" && stateStyles.unset]}>
            <Show when={properties.icon}>{(icon) => <LazyIcon name={icon()} />}</Show>
            <span {...style.attrs(stateStyles.name)}>{properties.label}</span>
        </FieldLabel>
    );

    return (
        <Show when={properties.description} fallback={label()}>
            {(said) => (
                <Tooltip>
                    <TooltipTrigger
                        render={(attributes) => (
                            <span {...attributes} {...style.attrs(stateStyles.trigger)}>
                                {label()}
                            </span>
                        )}
                    />
                    <TooltipContent>{said()}</TooltipContent>
                </Tooltip>
            )}
        </Show>
    );
}

/** Render the button that gives a value back to the one in force. */
export function ValueReset(properties: {
    /** What the value is called. */
    readonly label: string;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Give the value back. */
    readonly onReset: () => void;
}): JSX.Element {
    const locale = useLocale();

    return (
        <Button
            variant="ghost"
            size="sm"
            isIconOnly
            aria-label={locale.render(t.context("action")`Reset ${properties.label}`)}
            disabled={properties.disabled}
            onClick={() => properties.onReset()}
        >
            <Icon icon={resetIcon} />
        </Button>
    );
}

/** Render the mark of values several objects hold apart, which a press turns into the input. */
export function ValueMixed(properties: {
    /** What the value is called, where no field's label says so. */
    readonly "aria-label"?: string | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Start editing every object's value at once. */
    readonly onEdit: () => void;
}): JSX.Element {
    // take the field's id, so its label names the mark as it names the input after it
    const locale = useLocale();
    const field = useFieldControl();

    return (
        <Button
            variant="outline"
            id={field?.id}
            aria-label={properties["aria-label"]}
            aria-describedby={field?.attributes()["aria-describedby"]}
            disabled={properties.disabled}
            xstyle={stateStyles.mixed}
            onClick={() => properties.onEdit()}
        >
            {locale.render(t.context("option")`Mixed`)}
        </Button>
    );
}
