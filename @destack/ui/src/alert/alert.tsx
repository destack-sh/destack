import * as style from "@destack/style";
import { color, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createContext, type JSX, merge, omit, useContext } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The variant of an alert that sets none. */
const DEFAULTS: Required<Pick<AlertProperties, "variant">> = { variant: "default" };

/** The styles of an alert and its elements. */
const styles = style.create({
    alert: {
        position: "relative",
        display: "grid",
        gridTemplateColumns: { default: "0 1fr", ":has(> svg)": "auto 1fr" },
        alignItems: "start",
        columnGap: { default: 0, ":has(> svg)": space[3] },
        rowGap: space[1],
        width: "100%",
        paddingInline: space[4],
        paddingBlock: space[3],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.card,
    },
    title: {
        gridColumnStart: 2,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        fontWeight: weight.medium,
    },
    description: {
        gridColumnStart: 2,
        display: "grid",
        justifyItems: "start",
        gap: space[1],
        color: color.mutedForeground,
    },
});

/** The colors of each variant. */
const variants = style.create({
    default: {
        color: color.cardForeground,
    },
    destructive: {
        color: color.destructive,
    },
});

/** The colors of each variant's description. */
const descriptions = style.create({
    default: {},
    destructive: {
        color: `color-mix(in oklab, ${color.destructive} 90%, transparent)`,
    },
});

/** The variant of the nearest alert, which its description follows. */
const AlertContext = createContext<() => AlertVariant>(() => "default");

/** The look of an alert, by how urgent it is. */
export type AlertVariant = "default" | "destructive";

/** The properties of an element of an alert, the native element's attributes included. */
export type AlertElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of an alert, the native element's attributes included. */
export interface AlertProperties extends AlertElementProperties {
    /** The look, default by default. */
    readonly variant?: AlertVariant;
}

/** Render a message that calls for attention, with an optional leading icon, announced as an alert. */
export function Alert(properties: AlertProperties): JSX.Element {
    const alert = merge(DEFAULTS, properties);
    const rest = omit(alert, "variant", "xstyle", "style");

    return (
        <AlertContext value={() => alert.variant}>
            <div
                data-slot="alert"
                data-variant={alert.variant}
                role="alert"
                {...rest}
                {...style.attributes(
                    [text.callout, styles.alert, variants[alert.variant], alert.xstyle],
                    alert.style,
                )}
            />
        </AlertContext>
    );
}

/** Render the title of an alert on one line. */
export function AlertTitle(properties: AlertElementProperties): JSX.Element {
    return renderPart("div", "alert-title", properties, styles.title);
}

/** Render the description under an alert's title. */
export function AlertDescription(properties: AlertElementProperties): JSX.Element {
    const variant = useContext(AlertContext);

    return renderPart("div", "alert-description", properties, () => [
        styles.description,
        descriptions[variant()],
    ]);
}
