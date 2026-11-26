import { field } from "@destack/object";
import arrowUpIcon from "@destack/icon/phosphor/arrow-up";
import stopIcon from "@destack/icon/phosphor/stop";
import { Shortcut, t } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, motion, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import { createContext, type JSX, omit, useContext, useLocale } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { Textarea, type TextareaProperties } from "../textarea/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The tallest the text grows before it scrolls. */
const TALLEST = `calc(48 * ${space[1]})`;

/** The corners of the frame, a padding rounder than the cards inside, square where they are. */
const CORNER = `calc(5 / 3 * ${radius[5]})`;

/** The styles of a composer and its parts. */
const styles = style.create({
    composer: {
        width: "100%",
        minWidth: 0,
    },
    // stand the header over the text, the footer below it, and addons beside it
    frame: {
        display: "grid",
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        gridTemplateAreas: '"header header header" "start text end" "footer footer footer"',
        alignItems: "end",
        minWidth: 0,
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: {
            default: color.border,
            ":has([data-slot=composer-input]:focus)": color.ring,
        },
        color: color.foreground,
        cursor: { default: "text", ":is([data-disabled])": "not-allowed" },
        opacity: { default: 1, ":is([data-disabled])": 0.5 },
        transitionProperty: "border-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    input: {
        gridArea: "text",
        alignSelf: "center",
        boxSizing: "border-box",
        width: "100%",
        minWidth: 0,
        minHeight: size[2],
        maxHeight: TALLEST,
        paddingBlock: `calc((${size[2]} - 1lh) / 2)`,
        paddingInline: space[2],
        color: "inherit",
        fontFamily: "inherit",
        // write in the frame's size where it is smaller
        fontSize: { default: null, ":is([data-size=sm] > *)": "inherit" },
        overflowY: "auto",
    },
    header: {
        gridArea: "header",
        display: { default: "flex", ":empty": "none" },
        flexWrap: "wrap",
        alignItems: "flex-start",
        gap: space[2],
        minWidth: 0,
        marginBlockEnd: space[2],
        cursor: "default",
    },
    footer: {
        gridArea: "footer",
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
        marginBlockStart: space[1],
        cursor: "default",
    },
    addon: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        cursor: "default",
    },
    start: { gridArea: "start" },
    end: { gridArea: "end" },
    submit: {
        marginInlineStart: "auto",
        transitionProperty: "opacity, background-color, transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
});

/** The room and corners of each size. */
const sizes = style.create({
    default: {
        padding: space[2],
        borderRadius: CORNER,
    },
    sm: {
        padding: space[1],
        borderRadius: radius[4],
    },
});

/** The surface of each frame. */
const frames = style.create({
    raised: {
        backgroundColor: color.card,
        boxShadow: shadow.raised,
    },
    muted: {
        borderColor: {
            default: "transparent",
            ":has([data-slot=composer-input]:focus)": color.ring,
        },
        backgroundColor: color.muted,
    },
    outline: {
        backgroundColor: "transparent",
    },
});

/** How a composer is sent by the keys: Enter, Enter with the command key, or not at all. */
export type ComposerSubmitMode = "enter" | "ctrlEnter" | "none";

/** How a composer is sent and stopped, which any text in it uses. */
export class ComposerControl {
    /** The composer's form. */
    form: HTMLFormElement | undefined;
    /** The composer's properties. */
    readonly #properties: ComposerProperties;

    /** Follow a composer's properties. */
    constructor(properties: ComposerProperties) {
        this.#properties = properties;
    }

    /** Read how the keys send it. */
    submitMode(): ComposerSubmitMode {
        return this.#properties.submitMode ?? "enter";
    }

    /** Report whether an answer is under way. */
    isBusy(): boolean {
        return this.#properties.busy === true;
    }

    /** Report whether it takes nothing. */
    isDisabled(): boolean {
        return this.#properties.disabled === true;
    }

    /** Send what is written, unless its button is disabled, as a form's own Enter does. */
    submit(): void {
        const button = this.form?.querySelector<HTMLButtonElement>("button[type=submit]");
        if (button?.disabled !== true) {
            this.form?.requestSubmit();
        }
    }

    /** Stop the answer under way. */
    stop(): void {
        this.#properties.onStop?.();
    }

    /** Send or stop by a key pressed in its text, a text area or any editor. */
    press(event: KeyboardEvent): void {
        // read whether Enter is pressed, and what with
        const isTouch = event.view?.matchMedia("(pointer: coarse)").matches === true;
        const isHeld = event.shiftKey || event.altKey || event.metaKey || event.ctrlKey;
        const isEnter = event.key === "Enter" && !event.isComposing && !event.defaultPrevented;
        const isCommand = Shortcut.isCommand(navigator.userAgent);
        const isSubmit = Shortcut.matches("mod+enter", event, isCommand);
        const mode = this.submitMode();
        const isSent = (mode === "enter" && !isHeld && !isTouch) || (mode !== "none" && isSubmit);

        // send by Enter, alone or with the command key, as the composer is sent
        if (isEnter && isSent) {
            event.preventDefault();
            this.submit();
        }
        // keep a text area from sending where no key does
        else if (isEnter && isSubmit) {
            event.preventDefault();
        }
        // stop the answer under way by Escape
        else if (event.key === "Escape" && this.isBusy() && !event.defaultPrevented) {
            event.preventDefault();
            this.stop();
        }
    }
}

/** The nearest composer, null outside one. */
const ComposerContext = createContext<ComposerControl | null>(null);

/** The properties of an element of a composer, the native element's attributes included. */
export type ComposerElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of a composer, the native form's attributes included. */
export type ComposerProperties = Omit<
    JSX.FormHTMLAttributes<HTMLFormElement>,
    "class" | "onSubmit"
> &
    ElementPartProperties & {
        /** Send what is written. */
        readonly onSubmit?: (event: SubmitEvent) => void;
        /** How the keys send it, by Enter alone by default. */
        readonly submitMode?: ComposerSubmitMode;
        /** Whether an answer is under way, which the button stops in place of sending. */
        readonly busy?: boolean;
        /** Stop the answer under way. */
        readonly onStop?: () => void;
        /** Whether it takes nothing. */
        readonly disabled?: boolean;
    };

/** Render the box a message is written in, as a form around its parts. */
export function Composer(properties: ComposerProperties): JSX.Element {
    // send by the form, so its fields, drafts and errors work as any other's
    const control = new ComposerControl(properties);
    const rest = omit(properties, "onSubmit", "submitMode", "busy", "onStop", "disabled");

    return (
        <ComposerContext value={control}>
            {renderPart("form", "composer", rest, styles.composer, {
                ref: (form: HTMLFormElement) => (control.form = form),
                get "data-busy"() {
                    return properties.busy === true ? "" : undefined;
                },
                get "data-disabled"() {
                    return properties.disabled === true ? "" : undefined;
                },
                onSubmit: (event: SubmitEvent) => {
                    event.preventDefault();
                    if (!control.isDisabled()) {
                        properties.onSubmit?.(event);
                    }
                },
            })}
        </ComposerContext>
    );
}

/** The properties people change in a composer frame's controls and inspector. */
const COMPOSER_FRAME_PROPERTIES = {
    variant: field
        .enum({
            raised: { title: t.context("option")`Raised` },
            muted: { title: t.context("option")`Muted` },
            outline: { title: t.context("option")`Outline` },
        })
        .labeled(t.context("label")`Variant`)
        .described(t`Whether the frame is raised from the page, filled, or drawn by its edge.`)
        .default("raised"),
    size: field
        .enum({
            default: { title: t.context("option")`Default` },
            sm: { title: t.context("option")`Small` },
        })
        .labeled(t.context("label")`Size`)
        .described(t`A chat's size, or smaller for a comment.`)
        .default("default"),
};

/** Render the frame of a composer around its parts, whose free room goes to the text. */
export const ComposerFrame = defineComponent({
    name: "composerFrame",
    properties: COMPOSER_FRAME_PROPERTIES,
    render(
        properties: RenderProperties<ComposerElementProperties, typeof COMPOSER_FRAME_PROPERTIES>,
    ): JSX.Element {
        // lay the parts out, and focus the text by a press between them
        const composer = useComposer();
        const rest = omit(properties, "variant", "size");

        return renderPart(
            "div",
            "composer-frame",
            rest,
            () => [text.sm, styles.frame, sizes[properties.size], frames[properties.variant]],
            {
                get "data-size"() {
                    return properties.size;
                },
                get "data-variant"() {
                    return properties.variant;
                },
                get "data-disabled"() {
                    return composer.isDisabled() ? "" : undefined;
                },
                onPointerDown: (event: PointerEvent & { readonly currentTarget: HTMLElement }) => {
                    // write by a press between the parts
                    const { currentTarget, target } = event;
                    const isFree =
                        target === currentTarget ||
                        (target instanceof HTMLElement &&
                            target.dataset["slot"] === "composer-footer");
                    if (isFree) {
                        event.preventDefault();
                        currentTarget
                            .querySelector<HTMLElement>("[data-slot=composer-input]")
                            ?.focus();
                    }
                },
            },
        );
    },
});

/** The properties of a composer's frame, its native attributes included. */
export type ComposerFrameProperties = ComponentProperties<typeof ComposerFrame>;

/** Render the text of a composer, which Enter sends and Shift with Enter breaks. */
export function ComposerInput(properties: TextareaProperties): JSX.Element {
    // send and stop by its keys, whichever element it renders
    const composer = useComposer();
    const locale = useLocale();

    return (
        <Textarea
            variant="plain"
            data-slot="composer-input"
            rows={1}
            aria-label={locale.render(t.context("label")`Message`)}
            disabled={composer.isDisabled()}
            {...mergeProperties(
                { onKeyDown: (event: KeyboardEvent) => composer.press(event) },
                properties,
            )}
            xstyle={[text.base, styles.input, properties.xstyle]}
        />
    );
}

/** Render what stands over the text of a composer, such as what it answers and its files. */
export function ComposerHeader(properties: ComposerElementProperties): JSX.Element {
    return renderPart("div", "composer-header", properties, styles.header);
}

/** Render what stands below the text of a composer, such as its buttons. */
export function ComposerFooter(properties: ComposerElementProperties): JSX.Element {
    return renderPart("div", "composer-footer", properties, styles.footer);
}

/** The properties people change in a composer addon's controls and inspector. */
const COMPOSER_ADDON_PROPERTIES = {
    align: field
        .enum({
            "inline-start": { title: t.context("option")`Start` },
            "inline-end": { title: t.context("option")`End` },
        })
        .labeled(t.context("label")`Align`)
        .described(t`The side of the text it stands at.`)
        .default("inline-end"),
};

/** Render what stands beside the text of a composer, as in one that takes one row. */
export const ComposerAddon = defineComponent({
    name: "composerAddon",
    properties: COMPOSER_ADDON_PROPERTIES,
    render(
        properties: RenderProperties<ComposerElementProperties, typeof COMPOSER_ADDON_PROPERTIES>,
    ): JSX.Element {
        const rest = omit(properties, "align");
        const isStart = (): boolean => properties.align === "inline-start";

        return renderPart(
            "div",
            "composer-addon",
            rest,
            () => [styles.addon, isStart() ? styles.start : styles.end],
            {
                get "data-align"() {
                    return properties.align;
                },
            },
        );
    },
});

/** The properties of what stands beside a composer's text, its native attributes included. */
export type ComposerAddonProperties = ComponentProperties<typeof ComposerAddon>;

/** Render the button that sends a composer, or stops the answer under way. */
export function ComposerSubmit(properties: ButtonProperties): JSX.Element {
    // send or stop
    const composer = useComposer();
    const locale = useLocale();
    const label = (): string =>
        composer.isBusy()
            ? locale.render(t.context("action")`Stop`)
            : locale.render(t.context("action")`Send`);

    return (
        <Button
            isIconOnly
            data-slot="composer-submit"
            type={composer.isBusy() ? "button" : "submit"}

            shape="round"
            aria-label={label()}
            disabled={composer.isDisabled()}
            {...mergeProperties(
                { onClick: () => (composer.isBusy() ? composer.stop() : undefined) },
                properties,
            )}
            xstyle={[styles.submit, properties.xstyle]}
        >
            {contentOr(properties, () => (
                <Icon icon={composer.isBusy() ? stopIcon : arrowUpIcon} />
            ))}
        </Button>
    );
}

/** Read the nearest composer, refusing a part outside one. */
export function useComposer(): ComposerControl {
    const composer = useContext(ComposerContext);
    if (composer === null) {
        throw new TypeError("a composer part needs a composer around it");
    }

    return composer;
}
