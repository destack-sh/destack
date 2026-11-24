import prohibit from "@destack/icon/phosphor/prohibit";
import uploadSimple from "@destack/icon/phosphor/upload-simple";
import { type Localization, t } from "@destack/locale";
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
import {
    createEffect,
    createSignal,
    For,
    handle,
    type JSX,
    merge,
    omit,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import { createDropzone } from "@destack/view/primitives/upload";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { attachFiles } from "../action/index.ts";
import { useFieldControl } from "../field/control.ts";
import {
    describeLimits,
    describeRefusal,
    type FileLimits,
    type Refusal,
} from "../file-item/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";
import {
    DropzoneContext,
    DropzoneControl,
    type DropzoneOptions,
    fileOf,
    useDropzone,
} from "./control.ts";

/** What a zone that sets nothing takes, and how it looks. */
const DEFAULTS: Required<
    Pick<DropzoneProperties, "limits" | "variant" | "size" | "multiple" | "disabled" | "target">
> = {
    limits: {},
    variant: "outline",
    size: "default",
    multiple: false,
    disabled: false,
    target: "zone",
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
        backgroundColor: `color-mix(in oklab, ${swatch.accentMark} 8%, ${color.background})`,
    },
    rejecting: {
        borderColor: { default: swatch.destructiveMark, [media.forcedColors]: "CanvasText" },
        backgroundColor: "transparent",
        color: swatch.destructiveText,
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
        borderRadius: radius[3],
        backgroundColor: `color-mix(in oklab, ${swatch.accentMark} 8%, ${color.background})`,
        fontWeight: weight.medium,
        pointerEvents: "none",
    },
    overlayPage: {
        position: "fixed",
        zIndex: 50,
        inset: space[3],
        gap: space[3],
        borderWidth: stroke.ring,
        borderRadius: radius[5],
        boxShadow: `0 0 0 ${space[3]} ${color.background}`,
    },
    overlayReject: {
        borderColor: { default: swatch.destructiveMark, [media.forcedColors]: "Highlight" },
        backgroundColor: `color-mix(in oklab, ${color.background} 92%, transparent)`,
        color: swatch.destructiveText,
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

/** Where files may be dropped and pasted: on the zone, or anywhere on the page. */
export type DropzoneTarget = "zone" | "page";

/** The properties of an element of a zone, with its own attributes. */
export type DropzoneElementProperties<Element extends HTMLElement = HTMLDivElement> = Omit<
    JSX.HTMLAttributes<Element>,
    "class"
> &
    ElementPartProperties;

/** The properties of a zone, with its element's attributes. */
export interface DropzoneProperties
    extends
        Omit<DropzoneElementProperties, "onError" | "onReset">,
        Omit<DropzoneOptions, "limits" | "multiple" | "disabled"> {
    /** The media types, sizes and number of files it takes, every file by default. */
    readonly limits?: FileLimits;
    /** Whether it takes several files, one by default. */
    readonly multiple?: boolean;
    /** Whether its form submits only with a file. */
    readonly required?: boolean;
    /** The id of the form that submits its files, the one around it by default. */
    readonly form?: string;
    /** Whether a person chooses and drops folders, whose files it takes. */
    readonly directory?: boolean;
    /** The camera a phone takes a picture with, in place of choosing a file. */
    readonly capture?: "user" | "environment";
    /** Handle dropped folders that cannot be read, or have more than 10,000 files. */
    readonly onError?: (error: unknown) => void;
    /** Where files may be dropped and pasted, on the zone by default. */
    readonly target?: DropzoneTarget;
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
    const field = useFieldControl();
    const given = merge(DEFAULTS, properties);
    const zone = merge(given, {
        get disabled() {
            return given.disabled || field?.isDisabled() === true;
        },
    });
    const rest = omit(
        zone,
        "onFiles",
        "onRefuse",
        "onError",
        "onReset",
        "validate",
        "files",
        "limits",
        "multiple",
        "name",
        "required",
        "form",
        "directory",
        "capture",
        "target",
        "variant",
        "size",
        "disabled",
        "children",
    );
    const dropzone = createDropzone({
        isReadingFolders: () => zone.directory === true,
        onDrop: (dropped) => control.take(dropped),
        onError: (error) => zone.onError?.(error),
    });
    const control = new DropzoneControl(zone, dropzone.isDragging, dropzone.offered);
    const paste = (event: ClipboardEvent): void => {
        // leave what a zone or a field inside took
        const pasted = [...(event.clipboardData?.files ?? [])];
        if (pasted.length > 0 && !event.defaultPrevented) {
            event.preventDefault();
            control.take(pasted.map((file) => fileOf(file)));
        }
    };
    const reset = (): void => control.reset();
    const isOutline = (): boolean => zone.variant === "outline";
    const isPage = (): boolean => zone.target === "page";

    // take files dropped and pasted anywhere on the page
    createEffect(isPage, (isWhole) => {
        if (!isWhole) {
            return undefined;
        }
        dropzone.ref(document.documentElement);
        document.addEventListener("paste", paste);

        return () => document.removeEventListener("paste", paste);
    });

    // submit the files there are, and forget them as the form resets
    createEffect(
        () => ({ files: control.files, name: zone.name }),
        ({ files, name }) => (name === undefined ? undefined : control.submit(files)),
    );
    createEffect(
        () => ({ name: zone.name, form: zone.form }),
        ({ name }) => {
            // listen to the form of a zone with a name
            const form = name === undefined ? null : (input?.form ?? null);
            form?.addEventListener("reset", reset);

            return () => form?.removeEventListener("reset", reset);
        },
    );

    // tell the files added and removed
    const locale = useLocale();
    const [announcement, setAnnouncement] = createSignal("", { ownedWrite: true });
    let known = untrack(() => control.files);
    createEffect(
        () => control.files,
        (files) => {
            // compare with the files before
            setAnnouncement(changeOf(files, known, locale));
            known = files;
        },
    );
    let input: HTMLInputElement | undefined;

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
            isOutline() && control.drag === "reject" && styles.rejecting,
            zone.disabled && styles.disabled,
        ],
        {
            ref: [
                (element: HTMLElement) => (isPage() ? undefined : dropzone.ref(element)),
                handle(attachFiles, {
                    isEnabled: () => !zone.disabled,
                    run: () => control.choose(),
                }),
            ],
            get "data-variant"() {
                return zone.variant;
            },
            get "data-size"() {
                return zone.size;
            },
            get "data-dragging"() {
                return control.isDragging ? "" : undefined;
            },
            get "data-drag"() {
                return control.drag;
            },
            onPaste: (event: ClipboardEvent) => (isPage() ? undefined : paste(event)),
            get children() {
                return (
                    <>
                        <input
                            ref={(element) => {
                                input = element;
                                control.attach(element);
                            }}
                            type="file"
                            tabindex={-1}
                            aria-hidden={zone.required === true ? undefined : "true"}
                            aria-label={
                                zone.required === true ? titleOf(control, locale) : undefined
                            }
                            accept={zone.limits.accept?.join(",") ?? ""}
                            multiple={zone.multiple}
                            name={zone.name}
                            form={zone.form}
                            required={zone.required === true}
                            disabled={zone.disabled}
                            capture={zone.capture}
                            webkitdirectory={zone.directory === true}
                            onChange={() => control.read()}
                            {...style.attrs(visuallyHiddenStyle())}
                        />
                        <span role="status" {...style.attrs(visuallyHiddenStyle())}>
                            {announcement()}
                        </span>
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
    const field = useFieldControl();

    return renderPart("button", "dropzone-open", properties, [state.focus, styles.open], {
        type: "button",
        get id() {
            return field?.id;
        },
        get "aria-labelledby"() {
            return field === null ? undefined : `${field.labelId} ${field.id}`;
        },
        get "aria-describedby"() {
            return field?.attributes()["aria-describedby"];
        },
        get "aria-invalid"() {
            return field?.attributes()["aria-invalid"];
        },
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
    const zone = useDropzone();

    return renderPart("div", "dropzone-media", properties, styles.frame, {
        get children() {
            return (
                properties.children ?? (
                    <Icon icon={zone.drag === "reject" ? prohibit : uploadSimple} />
                )
            );
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
            return (
                properties.children ??
                (zone.drag === "reject"
                    ? locale.render(t`These files do not fit`)
                    : titleOf(zone, locale))
            );
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

/** The properties of what stands over a zone, with its element's attributes. */
export interface DropzoneOverlayProperties extends DropzoneElementProperties {
    /** What it stands over, the zone by default. */
    readonly target?: DropzoneTarget;
}

/** Render what stands over a zone or the page while files are dragged over it. */
export function DropzoneOverlay(properties: DropzoneOverlayProperties): JSX.Element {
    // tell by the drag whether the files fit
    const zone = useDropzone();
    const locale = useLocale();
    const isRejecting = (): boolean => zone.drag === "reject";

    return (
        <Show when={zone.isDragging}>
            {renderPart(
                "div",
                "dropzone-overlay",
                omit(properties, "target"),
                () => [
                    properties.target === "page" ? text.lg : text.sm,
                    styles.overlay,
                    properties.target === "page" && styles.overlayPage,
                    isRejecting() && styles.overlayReject,
                ],
                {
                    get children() {
                        return (
                            properties.children ?? (
                                <>
                                    <Icon icon={isRejecting() ? prohibit : uploadSimple} />
                                    {locale.render(
                                        isRejecting() ? t`These files do not fit` : t`Drop to add`,
                                    )}
                                </>
                            )
                        );
                    },
                },
            )}
        </Show>
    );
}

/** Write which files were added, else removed, empty for no change. */
function changeOf(files: readonly File[], before: readonly File[], locale: Localization): string {
    // compare the files
    const added = files.filter((file) => !before.includes(file));
    const removed = before.filter((file) => !files.includes(file));
    const names = (changed: readonly File[]) => locale.list(changed.map(({ name }) => name));
    if (added.length > 0) {
        return locale.render(t`Added ${names(added)}`);
    }

    return removed.length > 0 ? locale.render(t`Removed ${names(removed)}`) : "";
}

/** Write what a zone asks for. */
function titleOf(zone: DropzoneControl, locale: Localization): string {
    return locale.render(
        zone.isMultiple ? t`Drop files here, or choose` : t`Drop a file here, or choose`,
    );
}
