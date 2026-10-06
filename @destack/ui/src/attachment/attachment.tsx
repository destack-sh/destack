import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { triggerMarker } from "./marker.stylex.ts";
import {
    color,
    motion,
    radius,
    size,
    space,
    stroke,
    weight,
    width,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createContext, type JSX, merge, omit, useContext } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";

/** The state, size and orientation of an attachment that sets none. */
const DEFAULTS: Required<Pick<AttachmentProperties, "state" | "size" | "orientation">> = {
    state: "done",
    size: "default",
    orientation: "horizontal",
};

/** The variant of an attachment's media that sets none. */
const MEDIA_DEFAULTS: Required<Pick<AttachmentMediaProperties, "variant">> = { variant: "icon" };

/** The variant, size and type of an attachment's action that sets none. */
const ACTION_DEFAULTS: Required<Pick<ButtonProperties, "variant" | "size" | "type">> = {
    variant: "ghost",
    size: "icon-xs",
    type: "button",
};

/** The fade an attachment's title repeats while it uploads or processes. */
const pulse = style.keyframes({
    "0%, 100%": { opacity: 1 },
    "50%": { opacity: 0.5 },
});

/** The styles of an attachment and its elements. */
const styles = style.create({
    attachment: {
        position: "relative",
        display: "flex",
        flexShrink: 0,
        flexWrap: "wrap",
        width: "fit-content",
        maxWidth: "100%",
        minWidth: 0,
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[5],
        backgroundColor: {
            default: color.card,
            [style.when.descendant(":hover", triggerMarker)]: {
                default: null,
                [media.hover]: `color-mix(in oklab, ${color.muted} 50%, ${color.card})`,
            },
        },
        color: color.cardForeground,
        transitionProperty: "background-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-within": "solid" },
        outlineWidth: stroke.border,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    grouped: {
        flex: "none",
        scrollSnapAlign: "start",
    },
    idle: {
        borderStyle: "dashed",
    },
    error: {
        borderColor: `color-mix(in oklab, ${color.destructive} 30%, transparent)`,
    },
    media: {
        position: "relative",
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        overflow: "hidden",
        aspectRatio: "1",
        borderRadius: radius[4],
        backgroundColor: color.muted,
        color: color.foreground,
    },
    mediaError: {
        backgroundColor: `color-mix(in oklab, ${color.destructive} 10%, transparent)`,
        color: color.destructive,
    },
    mediaPending: {
        opacity: 0.6,
    },
    content: {
        flex: 1,
        maxWidth: "100%",
        minWidth: 0,
        lineHeight: 1.25,
    },
    title: {
        display: "block",
        overflow: "hidden",
        maxWidth: "100%",
        minWidth: 0,
        fontWeight: weight.medium,
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    pending: {
        animationName: pulse,
        animationDuration: `calc(4 * ${motion.durationLong})`,
        animationTimingFunction: motion.easingStandard,
        animationIterationCount: "infinite",
    },
    description: {
        display: "block",
        overflow: "hidden",
        maxWidth: "100%",
        minWidth: 0,
        marginTop: space[1],
        color: color.mutedForeground,
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    descriptionError: {
        color: `color-mix(in oklab, ${color.destructive} 80%, transparent)`,
    },
    actions: {
        position: "relative",
        zIndex: 2,
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
    },
    actionsVertical: {
        position: "absolute",
        top: space[3],
        insetInlineEnd: space[3],
        gap: space[1],
    },
    trigger: {
        position: "absolute",
        inset: 0,
        zIndex: 1,
        padding: 0,
        borderWidth: 0,
        borderRadius: "inherit",
        backgroundColor: "transparent",
        cursor: "pointer",
        outlineStyle: "none",
    },
    group: {
        display: "flex",
        gap: space[3],
        minWidth: 0,
        overflowX: "auto",
        overscrollBehaviorX: "contain",
        paddingBlock: space[1],
        scrollPaddingInline: space[1],
        scrollSnapType: "x mandatory",
        scrollbarWidth: "none",
    },
});

/** The gap, padding and text of each size. */
const sizes = style.create({
    default: {
        gap: space[2],
        padding: space[2],
    },
    sm: {
        gap: space[2],
        padding: space[1],
    },
    xs: {
        gap: space[1],
        padding: space[1],
        borderRadius: radius[4],
    },
});

/** The text style of each size. */
const sizeTexts = { default: text.callout, sm: text.footnote, xs: text.caption } as const;

/** The media width of each size. */
const mediaSizes = style.create({
    default: { width: size[3] },
    sm: { width: size[2] },
    xs: { width: size[1], borderRadius: radius[3] },
});

/** The layout of each orientation. */
const orientations = style.create({
    horizontal: { alignItems: "center", minWidth: width.row },
    vertical: { flexDirection: "column", width: width.tile },
});

/** The state, size and orientation of the nearest attachment, which its elements follow. */
const AttachmentContext = createContext<AttachmentLayout>({
    state: () => "done",
    size: () => "default",
    orientation: () => "horizontal",
});

/** Whether an attachment sits in a group, snapping as the group scrolls. */
const AttachmentGroupContext = createContext(false);

/** The state, size and orientation an attachment shares with its elements. */
interface AttachmentLayout {
    /** The state of the file. */
    readonly state: () => AttachmentState;
    /** The size of the attachment. */
    readonly size: () => AttachmentSize;
    /** The orientation of the attachment. */
    readonly orientation: () => AttachmentOrientation;
}

/** Where an attached file is: waiting to upload, uploading, processing, failed or ready. */
export type AttachmentState = "idle" | "uploading" | "processing" | "error" | "done";

/** The size of an attachment. */
export type AttachmentSize = "default" | "sm" | "xs";

/** Whether an attachment lays its media beside its text or above it. */
export type AttachmentOrientation = "horizontal" | "vertical";

/** How an attachment's media frames its content: an icon on a tile, or a cropped picture. */
export type AttachmentMediaVariant = "icon" | "image";

/** The properties of an element of an attachment, the native element's attributes included. */
export type AttachmentElementProperties<Attributes = JSX.HTMLAttributes<HTMLDivElement>> = Omit<
    Attributes,
    "class"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of an attachment, the native element's attributes included. */
export interface AttachmentProperties extends AttachmentElementProperties {
    /** The state of the file, done by default. */
    readonly state?: AttachmentState;
    /** The size, default by default. */
    readonly size?: AttachmentSize;
    /** The orientation, horizontal by default. */
    readonly orientation?: AttachmentOrientation;
}

/** The properties of an attachment's media, the native element's attributes included. */
export interface AttachmentMediaProperties extends AttachmentElementProperties {
    /** The frame, icon by default. */
    readonly variant?: AttachmentMediaVariant;
}

/** Render a file attached to a message or note, with its state while it uploads. */
export function Attachment(properties: AttachmentProperties): JSX.Element {
    // share the attachment's layout and snap it inside a group
    const isGrouped = useContext(AttachmentGroupContext);
    const attachment = merge(DEFAULTS, properties);
    const rest = omit(attachment, "state", "size", "orientation", "xstyle", "style");
    const layout: AttachmentLayout = {
        state: () => attachment.state,
        size: () => attachment.size,
        orientation: () => attachment.orientation,
    };

    return (
        <AttachmentContext value={layout}>
            <div
                data-slot="attachment"
                data-state={attachment.state}
                data-size={attachment.size}
                data-orientation={attachment.orientation}
                aria-busy={
                    attachment.state === "uploading" || attachment.state === "processing"
                        ? "true"
                        : undefined
                }
                {...rest}
                {...style.attributes(
                    [
                        sizeTexts[attachment.size],
                        styles.attachment,
                        sizes[attachment.size],
                        orientations[attachment.orientation],
                        isGrouped && styles.grouped,
                        attachment.state === "idle" && styles.idle,
                        attachment.state === "error" && styles.error,
                        attachment.xstyle,
                    ],
                    attachment.style,
                )}
            />
        </AttachmentContext>
    );
}

/** Render an attachment's file icon or preview. */
export function AttachmentMedia(properties: AttachmentMediaProperties): JSX.Element {
    // size the media beside the text and fade a picture while it uploads
    const layout = useContext(AttachmentContext);
    const figure = merge(MEDIA_DEFAULTS, properties);
    const rest = omit(figure, "variant", "xstyle", "style");
    const mediaSize = (): style.Styles =>
        layout.orientation() === "horizontal" ? mediaSizes[layout.size()] : null;
    const isFaded = (): boolean =>
        figure.variant === "image" &&
        (layout.state() === "uploading" || layout.state() === "processing");

    return (
        <div
            data-slot="attachment-media"
            data-variant={figure.variant}
            {...rest}
            {...style.attributes(
                [
                    styles.media,
                    mediaSize(),
                    layout.state() === "error" && styles.mediaError,
                    isFaded() && styles.mediaPending,
                    figure.xstyle,
                ],
                figure.style,
            )}
        />
    );
}

/** Render an attachment's title and description. */
export function AttachmentContent(properties: AttachmentElementProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="attachment-content"
            {...rest}
            {...style.attributes([styles.content, properties.xstyle], properties.style)}
        />
    );
}

/** Render an attachment's file name on one line, fading while the file uploads or processes. */
export function AttachmentTitle(
    properties: AttachmentElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    // fade the title while the file uploads or processes
    const layout = useContext(AttachmentContext);
    const rest = omit(properties, "xstyle", "style");
    const isPending = (): boolean =>
        layout.state() === "uploading" || layout.state() === "processing";

    return (
        <span
            data-slot="attachment-title"
            {...rest}
            {...style.attributes(
                [styles.title, isPending() && styles.pending, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render an attachment's size, type or error on one line. */
export function AttachmentDescription(
    properties: AttachmentElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    const layout = useContext(AttachmentContext);
    const rest = omit(properties, "xstyle", "style");

    return (
        <span
            data-slot="attachment-description"
            {...rest}
            {...style.attributes(
                [
                    text.caption,
                    styles.description,
                    layout.state() === "error" && styles.descriptionError,
                    properties.xstyle,
                ],
                properties.style,
            )}
        />
    );
}

/** Render an attachment's actions above its trigger. */
export function AttachmentActions(properties: AttachmentElementProperties): JSX.Element {
    const layout = useContext(AttachmentContext);
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="attachment-actions"
            {...rest}
            {...style.attributes(
                [
                    styles.actions,
                    layout.orientation() === "vertical" && styles.actionsVertical,
                    properties.xstyle,
                ],
                properties.style,
            )}
        />
    );
}

/** Render an action on an attachment, such as removing it, ghost and extra small unless set otherwise. */
export function AttachmentAction(properties: ButtonProperties): JSX.Element {
    const action = merge(ACTION_DEFAULTS, properties);

    return <Button data-slot="attachment-action" {...action} />;
}

/** Render a button covering an attachment that opens its file, named by its own label. */
export function AttachmentTrigger(
    properties: AttachmentElementProperties<JSX.ButtonHTMLAttributes<HTMLButtonElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <button
            type="button"
            data-slot="attachment-trigger"
            {...rest}
            {...style.attributes(
                [styles.trigger, triggerMarker, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render attachments in a row that scrolls sideways, snapping to each. */
export function AttachmentGroup(properties: AttachmentElementProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <AttachmentGroupContext value={true}>
            <div
                data-slot="attachment-group"
                {...rest}
                {...style.attributes([styles.group, properties.xstyle], properties.style)}
            />
        </AttachmentGroupContext>
    );
}
