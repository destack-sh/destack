import * as style from "@destack/style";
import { color, motion, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";
import { type CheckProperties, useCheckControl } from "../checkbox/check.ts";
import { useFieldControl } from "../field/control.ts";

/** The styles of a switch. */
const styles = style.create({
    switch: {
        appearance: "none",
        position: "relative",
        flexShrink: 0,
        boxSizing: "content-box",
        width: `calc(${size[2]} - 2 * ${stroke.border})`,
        height: space[4],
        margin: 0,
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius.full,
        backgroundColor: { default: color.input, ":checked": color.primary },
        boxShadow: shadow.inset,
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        transitionProperty: "background-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        "::before": {
            content: '""',
            position: "absolute",
            top: 0,
            left: 0,
            width: space[4],
            height: space[4],
            borderRadius: radius.full,
            backgroundColor: color.background,
            transform: {
                default: "none",
                ":checked": `translateX(calc(${size[2]} - 2 * ${stroke.border} - ${space[4]}))`,
            },
            transitionProperty: "transform",
            transitionDuration: motion.durationShort,
            transitionTimingFunction: motion.easingStandard,
        },
    },
});

/** The properties of a switch, the native checkbox's attributes included. */
export interface SwitchProperties
    extends
        Omit<
            JSX.InputHTMLAttributes<HTMLInputElement>,
            "class" | "type" | "role" | "checked" | "defaultChecked" | "onChange"
        >,
        CheckProperties {
    /** The StyleX styles applied after the switch's styles. */
    readonly xstyle?: style.Styles;
}

/** Render a native checkbox exposed as an on and off switch, which Space toggles. */
export function Switch(properties: SwitchProperties): JSX.Element {
    // take the id and state of the nearest field
    const field = useFieldControl();
    const checked = useCheckControl(properties);
    const rest = omit(
        properties,
        "checked",
        "defaultChecked",
        "onCheckedChange",
        "xstyle",
        "style",
    );

    return (
        <input
            type="checkbox"
            role="switch"
            data-slot="switch"
            checked={checked.isChecked()}
            {...field?.attributes()}
            {...rest}
            onChange={checked.onChange}
            {...style.attributes([styles.switch, properties.xstyle], properties.style)}
        />
    );
}
