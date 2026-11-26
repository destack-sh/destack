import * as style from "@destack/style";
import { color, radius, space, stroke, weight, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import x from "@destack/icon/phosphor/x";
import { t } from "@destack/locale";
import { type JSX, merge, omit, useLocale } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import { contentOr, type ElementPartProperties, renderPart } from "../part/index.ts";

/** The width an alert takes inside a parent sized to its content, which its queries cannot read. */
const INTRINSIC_WIDTH = "28rem";

/** The query of an alert too narrow for its actions beside its text, which go below it. */
const NARROW_QUERY = "@container alert (max-width: 28rem)";

/** The selector of an alert's end element when the alert has a description, which it spans to. */
const DESCRIBED = ":is([data-slot=alert]:has(> [data-slot=alert-description]) > *)";

/** The selector of an alert's element when the alert leads with an icon, which the text clears. */
const WITH_ICON = ":is([data-slot=alert]:has(> svg) > *)";

/** The selector of an alert's element when the alert has a title, which the description follows. */
const TITLED = ":is([data-slot=alert]:has(> [data-slot=alert-title]) > *)";

/** The variant of an alert that sets none. */
const DEFAULTS: Required<Pick<AlertProperties, "variant">> = { variant: "default" };

/** The styles of an alert and its elements. */
const styles = style.create({
    alert: {
        position: "relative",
        containerName: "alert",
        containerType: "inline-size",
        containIntrinsicInlineSize: `auto ${INTRINSIC_WIDTH}`,
        display: "grid",
        gridTemplateColumns: { default: "0 1fr auto auto", ":has(> svg)": "auto 1fr auto auto" },
        // center a leading icon on the first line of the text
        gridTemplateRows: "1lh auto auto",
        alignItems: "center",
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
        gridRow: "1 / span 2",
        minWidth: 0,
        paddingInlineStart: { default: 0, [WITH_ICON]: space[3] },
        overflowWrap: "anywhere",
        fontWeight: weight.medium,
    },
    description: {
        gridColumnStart: 2,
        gridRow: { default: "1 / span 2", [TITLED]: "3" },
        marginBlockStart: { default: 0, [TITLED]: space[1] },
        paddingInlineStart: { default: 0, [WITH_ICON]: space[3] },
        display: "grid",
        justifyItems: "start",
        gap: space[1],
        color: color.mutedForeground,
    },
    action: {
        gridColumnStart: { default: 3, [NARROW_QUERY]: 2 },
        gridRow: {
            default: "1",
            [NARROW_QUERY]: "auto",
            [DESCRIBED]: { default: "1 / span 3", [NARROW_QUERY]: "auto" },
        },
        alignSelf: "center",
        display: "flex",
        flexWrap: "wrap",
        gap: space[2],
        marginInlineStart: { default: space[3], [NARROW_QUERY]: 0 },
        marginBlockStart: { default: 0, [NARROW_QUERY]: space[2] },
        paddingInlineStart: {
            default: 0,
            [NARROW_QUERY]: { default: 0, [WITH_ICON]: space[3] },
        },
    },
    close: {
        gridColumnStart: 4,
        gridRow: { default: "1", [DESCRIBED]: "1 / span 3" },
        alignSelf: "start",
        marginBlock: `calc(-1 * ${space[1]})`,
        marginInlineStart: space[2],
        marginInlineEnd: `calc(-1 * ${space[2]})`,
    },
});

/** The colors of each variant: a status variant's text and a border tinted by its mark. */
const variants = style.create({
    default: {
        color: color.foreground,
    },
    info: {
        borderColor: `color-mix(in oklab, ${swatch.infoMark} 40%, ${color.border})`,
        color: swatch.infoText,
    },
    success: {
        borderColor: `color-mix(in oklab, ${swatch.successMark} 40%, ${color.border})`,
        color: swatch.successText,
    },
    warning: {
        borderColor: `color-mix(in oklab, ${swatch.warningMark} 40%, ${color.border})`,
        color: swatch.warningText,
    },
    destructive: {
        borderColor: `color-mix(in oklab, ${swatch.destructiveMark} 40%, ${color.border})`,
        color: swatch.destructiveText,
    },
});

/** The role of each variant: an alert interrupts for a warning or a failure, a status waits. */
const ROLES: Readonly<Record<AlertVariant, "alert" | "status">> = {
    default: "status",
    info: "status",
    success: "status",
    warning: "alert",
    destructive: "alert",
};

/** The look of an alert, by how urgent it is. */
export type AlertVariant = "default" | "info" | "success" | "warning" | "destructive";

/** The properties of an element of an alert, the native element's attributes included. */
export type AlertElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of an alert, the native element's attributes included. */
export interface AlertProperties extends AlertElementProperties {
    /** The look, default by default. */
    readonly variant?: AlertVariant;
}

/** Render a message that calls for attention, with an optional leading icon. */
export function Alert(properties: AlertProperties): JSX.Element {
    const alert = merge(DEFAULTS, properties);
    const rest = omit(alert, "variant");

    return renderPart(
        "div",
        "alert",
        rest,
        () => [text.sm, styles.alert, variants[alert.variant]],
        {
            get "data-variant"() {
                return alert.variant;
            },
            get role() {
                return ROLES[alert.variant];
            },
        },
    );
}

/** Render the title of an alert. */
export function AlertTitle(properties: AlertElementProperties): JSX.Element {
    return renderPart("div", "alert-title", properties, styles.title);
}

/** Render the description under an alert's title. */
export function AlertDescription(properties: AlertElementProperties): JSX.Element {
    return renderPart("div", "alert-description", properties, styles.description);
}

/** Render the actions at the end of an alert, such as retry or upgrade. */
export function AlertAction(properties: AlertElementProperties): JSX.Element {
    return renderPart("div", "alert-action", properties, styles.action);
}

/** Render the button that dismisses an alert, which the owner hides on its click. */
export function AlertClose(properties: Omit<ButtonProperties, "variant" | "size">): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "children");

    return (
        <Button
            isIconOnly
            variant="ghost"
            size="sm"
            aria-label={locale.render(t.context("action")`Dismiss`)}
            data-slot="alert-close"
            {...rest}
            xstyle={[styles.close, properties.xstyle]}
        >
            {contentOr(properties, () => (
                <Icon icon={x} />
            ))}
        </Button>
    );
}
