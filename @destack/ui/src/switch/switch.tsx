import * as style from "@destack/style";
import { color, motion, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    type JSX,
    merge,
    omit,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import {
    ToggleInput,
    type ToggleFormProperties,
    type ToggleStateProperties,
    isSubmitted,
    ToggleState,
} from "../toggle-state/index.ts";
import { useFieldControl } from "../field/control.ts";
import {
    type ElementPartProperties,
    type PartAttributes,
    type Render,
    rendered,
    renderPart,
} from "../part/index.ts";

/** The distance a checked thumb travels: the track's inner width less the thumb. */
const TRAVEL = `calc(${size[2]} - 2 * ${stroke.border} - ${space[4]})`;

/** The styles of a switch and its thumb. */
const styles = style.create({
    switch: {
        display: "inline-flex",
        alignItems: "center",
        flexShrink: 0,
        boxSizing: "content-box",
        width: `calc(${size[2]} - 2 * ${stroke.border})`,
        height: space[4],
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius.full,
        backgroundColor: color.input,
        boxShadow: shadow.inset,
        cursor: { default: "pointer", ":disabled": "not-allowed" },
        opacity: { default: 1, ":disabled": 0.5 },
        transitionProperty: "background-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    checked: {
        backgroundColor: color.primary,
    },
    thumb: {
        display: "block",
        flexShrink: 0,
        width: space[4],
        height: space[4],
        borderRadius: radius.full,
        backgroundColor: color.background,
        pointerEvents: "none",
        transform: "none",
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    thumbChecked: {
        transform: `translateX(${TRAVEL})`,
    },
    thumbCheckedReversed: {
        transform: `translateX(calc(-1 * ${TRAVEL}))`,
    },
});

/** The checked state and availability of the nearest switch, which its thumb reads. */
interface SwitchControl {
    /** Whether the switch is on. */
    readonly isChecked: Accessor<boolean>;
    /** Whether the switch ignores the person. */
    readonly isDisabled: Accessor<boolean>;
}

/** The nearest switch, null outside one. */
const SwitchContext = createContext<SwitchControl | null>(null);

/** The properties of a switch, the native button's attributes included. */
export interface SwitchProperties
    extends
        Omit<
            JSX.ButtonHTMLAttributes<HTMLButtonElement>,
            "class" | "type" | "role" | "name" | "value" | "form" | "onClick"
        >,
        ToggleStateProperties,
        ToggleFormProperties {
    /** The StyleX styles applied after the switch's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the switch's attributes, the native button by default. */
    readonly render?: Render;
}

/** The properties of a switch's thumb, the native element's attributes included. */
export type SwitchThumbProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> &
    ElementPartProperties;

/** Render a button exposed as an on and off switch around its thumb, which a click, Space or Enter flips and a form submits through a hidden checkbox. */
export function Switch(properties: SwitchProperties): JSX.Element {
    // take the id and state of the nearest field
    const field = useFieldControl();
    const toggle = new ToggleState(properties);
    const rest = omit(
        properties,
        "checked",
        "defaultChecked",
        "onCheckedChange",
        "name",
        "value",
        "required",
        "form",
        "disabled",
        "xstyle",
        "style",
        "render",
        "children",
    );
    const isDisabled = (): boolean => properties.disabled === true || field?.isDisabled() === true;

    // mark the part, report its state and flip it on activation
    const part: PartAttributes = merge(
        {
            role: "switch" as const,
            get "aria-checked"() {
                return toggle.isChecked() ? "true" : "false";
            },
            get "aria-required"() {
                return properties.required === true ? "true" : undefined;
            },
            "data-slot": "switch",
            get "data-state"() {
                return toggle.isChecked() ? "checked" : "unchecked";
            },
            get "data-disabled"() {
                return isDisabled() ? "" : undefined;
            },
            onClick: () => toggle.set(!toggle.isChecked()),
            get children() {
                return properties.children ?? <SwitchThumb />;
            },
        },
        () => ({ ...field?.attributes(), disabled: isDisabled() ? true : undefined }),
        () =>
            style.attributes(
                [styles.switch, toggle.isChecked() && styles.checked, properties.xstyle],
                properties.style,
            ),
    );

    return (
        <SwitchContext value={{ isChecked: toggle.isChecked, isDisabled }}>
            {rendered(properties.render, part, rest, () => (
                <button type="button" {...part} {...rest} />
            ))}
            <Show when={isSubmitted(properties)}>
                <ToggleInput
                    name={properties.name}
                    value={properties.value}
                    required={properties.required}
                    form={properties.form}
                    checked={toggle.isChecked()}
                    disabled={isDisabled()}
                    onReset={() => toggle.reset()}
                />
            </Show>
        </SwitchContext>
    );
}

/** Render the thumb of the nearest switch, which slides to the far end while the switch is on. */
export function SwitchThumb(properties: SwitchThumbProperties): JSX.Element {
    // read the switch and the text direction the thumb slides along
    const control = useContext(SwitchContext);
    if (control === null) {
        throw new TypeError("a switch thumb needs a switch around it");
    }
    const locale = useLocale();

    return renderPart(
        "span",
        "switch-thumb",
        properties,
        () => [
            styles.thumb,
            control.isChecked() &&
                (locale.direction === "rtl" ? styles.thumbCheckedReversed : styles.thumbChecked),
        ],
        {
            get "data-state"() {
                return control.isChecked() ? "checked" : "unchecked";
            },
            get "data-disabled"() {
                return control.isDisabled() ? "" : undefined;
            },
        },
    );
}
