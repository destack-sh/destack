import uploadSimple from "@destack/icon/phosphor/upload-simple";
import { type Localization, t } from "@destack/locale";
import type { ObjectFileLimits } from "@destack/object";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import {
    color,
    motion,
    radius,
    size,
    space,
    stroke,
    swatch,
    weight,
} from "@destack/theme/tokens.stylex";
import { Icon } from "@destack/ui/icon";
import { For, type JSX, merge, omit, Show, useLocale } from "@destack/view";
import { createDropzone } from "@destack/view/primitives/upload";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { DropzoneContext, DropzoneControl, useDropzone } from "./control.ts";
import { describeLimits, describeRefusal, type Refusal } from "../file-item/index.ts";

/** What a zone that sets nothing takes, and how it looks. */
const DEFAULTS: Required<
    Pick<DropzoneProperties, "limits" | "variant" | "size" | "multiple" | "disabled">
> = {
    limits: {},
    variant: "outline",
    size: "default",
    multiple: false,
    disabled: false,
};

/** The styles of a zone and its elements. */
const styles = style.create({
    zone: {
        position: "relative",
        isolation: "isolate",
        minWidth: 0,
    },
    outline: {
        display: "flex",
        alignItems: "center",
        borderWidth: stroke.border,
        borderStyle: "dashed",
        borderColor: color.input,
        borderRadius: radius[3],
        color: color.foreground,
        transitionProperty: "background-color, border-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    tall: {
        flexDirection: "column",
        justifyContent: "center",
        gap: space[3],
        padding: space[6],
        textAlign: "center",
    },
    row: {
        gap: space[3],
        padding: space[2],
    },
    opens: {
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.hover },
        },
    },
    dragging: {
        borderColor: { default: swatch.accentMark, [media.forcedColors]: "Highlight" },
        borderStyle: "solid",
        backgroundColor: swatch.accentTint,
    },
    disabled: {
        opacity: 0.5,
    },
    open: {
        position: "absolute",
        zIndex: 1,
        inset: 0,
        borderRadius: "inherit",
        cursor: { default: "pointer", ":disabled": "default" },
    },
    overlay: {
        position: "absolute",
        zIndex: 2,
        inset: 0,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: space[2],
        borderWidth: stroke.border,
        borderStyle: "dashed",
        borderColor: { default: swatch.accentMark, [media.forcedColors]: "Highlight" },
        borderRadius: "inherit",
        backgroundColor: `color-mix(in oklab, ${swatch.accentTint} 92%, transparent)`,
        fontWeight: weight.medium,
        pointerEvents: "none",
    },
    frame: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        width: size[3],
        height: size[3],
        borderRadius: radius[2],
        backgroundColor: color.muted,
        color: color.mutedForeground,
        fontSize: "1.25rem",
    },
    content: {
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
        textWrap: "balance",
    },
    title: {
        fontWeight: weight.medium,
    },
    hint: {
        color: color.mutedForeground,
    },
    line: {
        display: "block",
    },
    refusal: {
        overflowWrap: "anywhere",
        color: swatch.destructiveText,
    },
});

/** The look of a zone: a dashed box, or none, as around a page or a message's text. */
export type DropzoneVariant = "outline" | "plain";

/** The size of a zone: the default, or small in one row. */
export type DropzoneSize = "default" | "sm";

/** The properties of an element of a zone, with its own attributes. */
export type DropzoneElementProperties<Element extends HTMLElement = HTMLDivElement> = Omit<
    JSX.HTMLAttributes<Element>,
    "class"
> &
    ElementPartProperties;

/** The properties of a zone, with its element's attributes. */
export interface DropzoneProperties extends DropzoneElementProperties {
    /** Take the files dropped, pasted or chosen, those that fit the limits alone. */
    readonly onFiles: (files: readonly File[]) => void;
    /** Hear of the files that do not fit, which it tells by default too. */
    readonly onRefuse?: (refusals: readonly Refusal[]) => void;
    /** The media types and sizes it takes, such as a file field's own, every file by default. */
    readonly limits?: ObjectFileLimits;
    /** Whether it takes several files, one by default. */
    readonly multiple?: boolean;
    /** Its look, a dashed box by default. */
    readonly variant?: DropzoneVariant;
    /** The size of its box, default by default. */
    readonly size?: DropzoneSize;
    /** Whether it takes no file now. */
    readonly disabled?: boolean;
    /** Its parts, or what it stands around, the usual parts by default. */
    readonly children?: JSX.Element;
}

/** Render a zone that takes files dropped, pasted or chosen, and says why it refuses one. */
export function Dropzone(properties: DropzoneProperties): JSX.Element {
    // take every file, one at a time, unless it sets otherwise
    const zone = merge(DEFAULTS, properties);
    const rest = omit(
        zone,
        "onFiles",
        "onRefuse",
        "limits",
        "multiple",
        "variant",
        "size",
        "disabled",
        "children",
    );
    const dropzone = createDropzone({
        onDrop: (dropped) => control.take(dropped.map(({ file }) => file)),
    });
    const control = new DropzoneControl(zone, dropzone.isDragging);
    const paste = (event: ClipboardEvent): void => {
        const pasted = [...(event.clipboardData?.files ?? [])];
        if (pasted.length > 0) {
            event.preventDefault();
            control.take(pasted);
        }
    };
    const isOutline = (): boolean => zone.variant === "outline";

    return renderPart(
        "div",
        "dropzone",
        rest,
        () => [
            styles.zone,
            isOutline() && text.sm,
            isOutline() && styles.outline,
            isOutline() && (zone.size === "sm" ? styles.row : styles.tall),
            isOutline() && !zone.disabled && styles.opens,
            isOutline() && control.isDragging && styles.dragging,
            zone.disabled && styles.disabled,
        ],
        {
            ref: dropzone.ref,
            get "data-variant"() {
                return zone.variant;
            },
            get "data-size"() {
                return zone.size;
            },
            get "data-dragging"() {
                return control.isDragging ? "" : undefined;
            },
            onPaste: paste,
            get children() {
                return (
                    <>
                        <input
                            ref={(element) => control.attach(element)}
                            type="file"
                            hidden
                            accept={zone.limits.accept?.join(",") ?? ""}
                            multiple={zone.multiple}
                            onChange={() => control.read()}
                        />
                        <DropzoneContext value={control}>
                            <Show when={!("children" in properties)} fallback={properties.children}>
                                <DropzoneOpen />
                                <DropzoneMedia />
                                <DropzoneContent>
                                    <DropzoneTitle />
                                    <DropzoneHint />
                                    <DropzoneRefusal />
                                </DropzoneContent>
                            </Show>
                        </DropzoneContext>
                    </>
                );
            },
        },
    );
}

/** Render what opens the picker, over all of a zone. */
export function DropzoneOpen(
    properties: DropzoneElementProperties<HTMLButtonElement>,
): JSX.Element {
    // label it by what the zone asks for and takes
    const zone = useDropzone();
    const locale = useLocale();

    return renderPart("button", "dropzone-open", properties, [state.focus, styles.open], {
        type: "button",
        get "aria-label"() {
            return [titleOf(zone, locale), describeLimits(zone.limits, locale)]
                .filter((each) => each !== "")
                .join(". ");
        },
        get disabled() {
            return zone.isDisabled;
        },
        onClick: () => zone.choose(),
    });
}

/** Render a button that opens the picker, such as a paper clip beside a message's text. */
export function DropzoneTrigger(
    properties: DropzoneElementProperties<HTMLButtonElement>,
): JSX.Element {
    const zone = useDropzone();

    return renderPart("button", "dropzone-trigger", properties, null, {
        type: "button",
        get disabled() {
            return zone.isDisabled;
        },
        onClick: () => zone.choose(),
    });
}

/** Render the frame of a zone's icon. */
export function DropzoneMedia(properties: DropzoneElementProperties): JSX.Element {
    return renderPart("div", "dropzone-media", properties, styles.frame, {
        get children() {
            return properties.children ?? <Icon icon={uploadSimple} />;
        },
    });
}

/** Render a zone's title, hint and refusal. */
export function DropzoneContent(properties: DropzoneElementProperties): JSX.Element {
    return renderPart("div", "dropzone-content", properties, styles.content);
}

/** Render what a zone asks for, to drop or choose a file by default. */
export function DropzoneTitle(properties: DropzoneElementProperties<HTMLSpanElement>): JSX.Element {
    const zone = useDropzone();
    const locale = useLocale();

    return renderPart("span", "dropzone-title", properties, styles.title, {
        get children() {
            return properties.children ?? titleOf(zone, locale);
        },
    });
}

/** Render what a zone's limits take, nothing for one without. */
export function DropzoneHint(properties: DropzoneElementProperties<HTMLSpanElement>): JSX.Element {
    const zone = useDropzone();
    const locale = useLocale();

    return renderPart("span", "dropzone-hint", properties, [text.xs, styles.hint], {
        get children() {
            return properties.children ?? describeLimits(zone.limits, locale);
        },
    });
}

/** The properties of a zone's refusals, with their element's attributes. */
export interface DropzoneRefusalProperties extends Omit<
    DropzoneElementProperties<HTMLSpanElement>,
    "children"
> {
    /** Write the files it refused, each on its line by default. */
    readonly children?: (refusals: readonly Refusal[]) => JSX.Element;
}

/** Render why a zone refused files, told as it does. */
export function DropzoneRefusal(properties: DropzoneRefusalProperties): JSX.Element {
    // write each in the person's language
    const zone = useDropzone();
    const locale = useLocale();

    return renderPart(
        "span",
        "dropzone-refusal",
        omit(properties, "children"),
        [text.xs, styles.refusal],
        {
            role: "alert",
            get children() {
                return (
                    properties.children?.(zone.refusals) ?? (
                        <For each={zone.refusals}>
                            {(refusal) => (
                                <span {...style.attrs(styles.line)}>
                                    {describeRefusal(refusal, locale)}
                                </span>
                            )}
                        </For>
                    )
                );
            },
        },
    );
}

/** Render what stands over a zone while files are dragged over it. */
export function DropzoneOverlay(properties: DropzoneElementProperties): JSX.Element {
    const zone = useDropzone();
    const locale = useLocale();

    return (
        <Show when={zone.isDragging}>
            {renderPart("div", "dropzone-overlay", properties, [text.sm, styles.overlay], {
                get children() {
                    return (
                        properties.children ?? (
                            <>
                                <Icon icon={uploadSimple} />
                                {locale.render(t`Drop to add`)}
                            </>
                        )
                    );
                },
            })}
        </Show>
    );
}

/** Write what a zone asks for. */
function titleOf(zone: DropzoneControl, locale: Localization): string {
    return locale.render(
        zone.isMultiple ? t`Drop files here, or choose` : t`Drop a file here, or choose`,
    );
}
