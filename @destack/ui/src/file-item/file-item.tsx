import arrowClockwise from "@destack/icon/phosphor/arrow-clockwise";
import fileIcon from "@destack/icon/phosphor/file";
import fileArchive from "@destack/icon/phosphor/file-archive";
import fileAudio from "@destack/icon/phosphor/file-audio";
import fileCode from "@destack/icon/phosphor/file-code";
import fileImage from "@destack/icon/phosphor/file-image";
import filePdf from "@destack/icon/phosphor/file-pdf";
import fileText from "@destack/icon/phosphor/file-text";
import fileVideo from "@destack/icon/phosphor/file-video";
import fileXls from "@destack/icon/phosphor/file-xls";
import warning from "@destack/icon/phosphor/warning";
import xIcon from "@destack/icon/phosphor/x";
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
import { Icon, type IconProperties } from "@destack/ui/icon";
import {
    createContext,
    createSignal,
    type JSX,
    merge,
    omit,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";
import {
    FileItemContext,
    FileItemControl,
    type FileItemLayout,
    type FileItemOptions,
    useFileItem,
} from "./control.ts";
import type { FileKind } from "./kind.ts";
import { describeReason } from "./limits.ts";
import { FileSize } from "./size.ts";

/** The icon of each kind of file. */
const ICONS = {
    archive: fileArchive,
    audio: fileAudio,
    code: fileCode,
    image: fileImage,
    other: fileIcon,
    pdf: filePdf,
    sheet: fileXls,
    text: fileText,
    video: fileVideo,
} satisfies Record<FileKind, typeof fileIcon>;

/** The longest extension that names a file's format, as `.webmanifest` would not. */
const EXTENSION_LENGTH = 5;

/** The layout and size of a file that sets neither. */
const DEFAULTS: Required<Pick<FileItemProperties, "layout" | "size">> = {
    layout: "row",
    size: "default",
};

/** The share a ring shows as it turns. */
const STARTING_SHARE = 0.25;

/** The turn a ring repeats until the first part arrived. */
const turn = style.keyframes({
    from: { transform: "rotate(-90deg)" },
    to: { transform: "rotate(270deg)" },
});

/** The styles of a file and its elements. */
const styles = style.create({
    group: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
        minWidth: 0,
    },
    cards: {
        display: "grid",
        gridTemplateColumns: `repeat(auto-fill, minmax(min(9rem, calc(50% - ${space[2]})), 1fr))`,
        gap: space[3],
    },
    item: {
        position: "relative",
        isolation: "isolate",
        display: "flex",
        minWidth: 0,
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.card,
        color: color.foreground,
    },
    opens: {
        backgroundColor: {
            default: color.card,
            ":hover": { default: null, [media.hover]: color.hover },
        },
    },
    failed: {
        borderColor: swatch.destructiveMark,
    },
    row: {
        alignItems: "center",
        gap: space[3],
        padding: space[2],
    },
    rowSmall: {
        gap: space[2],
        padding: space[1],
    },
    card: {
        flexDirection: "column",
        padding: space[2],
        gap: space[2],
    },
    open: {
        position: "absolute",
        zIndex: 1,
        inset: 0,
        borderRadius: "inherit",
        cursor: "pointer",
    },
    frame: {
        position: "relative",
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        overflow: "hidden",
        borderRadius: radius[2],
        backgroundColor: color.muted,
        color: color.mutedForeground,
        fontSize: "1.25rem",
    },
    frameFailed: {
        color: swatch.destructiveText,
    },
    frameRow: {
        width: size[3],
        height: size[3],
    },
    frameRowSmall: {
        width: size[2],
        height: size[2],
        borderRadius: radius[1],
        fontSize: "1rem",
    },
    frameCard: {
        width: "100%",
        aspectRatio: "1",
        fontSize: "2rem",
    },
    picture: {
        width: "100%",
        height: "100%",
        objectFit: "cover",
    },
    waiting: {
        opacity: 0.5,
    },
    ring: {
        position: "absolute",
        inset: 0,
        margin: "auto",
        width: `min(${space[6]}, 60%)`,
        height: `min(${space[6]}, 60%)`,
        borderRadius: radius.full,
        color: color.foreground,
        transform: "rotate(-90deg)",
        fill: "none",
        stroke: "currentColor",
    },
    ringCard: {
        width: space[8],
        height: space[8],
    },
    turning: {
        animationName: { default: turn, [media.motionReduce]: "none" },
        animationDuration: `calc(2 * ${motion.durationLong})`,
        animationTimingFunction: "linear",
        animationIterationCount: "infinite",
    },
    track: {
        opacity: 0.2,
    },
    content: {
        display: "flex",
        flex: 1,
        flexDirection: "column",
        minWidth: 0,
    },
    contentCard: {
        flex: "none",
        paddingInline: space[1],
        paddingBlockEnd: space[1],
    },
    title: {
        overflow: "hidden",
        fontWeight: weight.medium,
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    description: {
        overflow: "hidden",
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    error: {
        color: swatch.destructiveText,
        overflowWrap: "anywhere",
        whiteSpace: "normal",
    },
    actions: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        gap: space[1],
    },
    actionsCard: {
        position: "absolute",
        zIndex: 2,
        insetBlockStart: space[3],
        insetInlineEnd: space[3],
    },
    action: {
        position: "relative",
        zIndex: 2,
    },
    actionCard: {
        borderColor: color.border,
        backgroundColor: color.card,
    },
});

/** How far a ring is drawn. */
const arcs = style.create({
    arc: (share: number) => ({ strokeDashoffset: `calc(1 - ${share})` }),
});

/** Whether a file sits in a group, listed as one of its entries. */
const FileItemGroupContext = createContext(false);

/** The properties of an element of a file, with its own attributes. */
export type FileItemElementProperties<Element extends HTMLElement = HTMLDivElement> = Omit<
    JSX.HTMLAttributes<Element>,
    "class"
> &
    ElementPartProperties;

/** The properties of a file, with its element's attributes. */
export interface FileItemProperties
    extends
        FileItemElementProperties,
        Omit<FileItemOptions, "layout" | "size">,
        Partial<Pick<FileItemOptions, "layout" | "size">> {
    /** Its parts, the usual ones by default. */
    readonly children?: JSX.Element;
}

/** Render a file of its parts: its picture or kind, name and size, and how far it is sent. */
export function FileItem(properties: FileItemProperties): JSX.Element {
    // follow the file, listed in its group
    const isGrouped = useContext(FileItemGroupContext);
    const item = merge(DEFAULTS, properties);
    const control = new FileItemControl(item);
    const rest = omit(
        item,
        "file",
        "source",
        "upload",
        "error",
        "layout",
        "size",
        "onOpen",
        "onRetry",
        "onRemove",
        "children",
    );
    const isCard = (): boolean => control.layout === "card";
    const isSmall = (): boolean => control.size === "sm";
    const isSmallRow = (): boolean => isSmall() && !isCard();

    return renderPart(
        "div",
        "file-item",
        rest,
        () => [
            isSmall() ? text.xs : text.sm,
            styles.item,
            isCard() ? styles.card : styles.row,
            isSmallRow() && styles.rowSmall,
            control.isOpenable && styles.opens,
            control.isFailed && styles.failed,
        ],
        {
            get "data-layout"() {
                return control.layout;
            },
            get "data-size"() {
                return control.size;
            },
            get "data-state"() {
                return control.status;
            },
            role: isGrouped ? "listitem" : undefined,
            get children() {
                return (
                    <FileItemContext value={control}>
                        <Show when={!("children" in properties)} fallback={properties.children}>
                            <FileItemOpen />
                            <FileItemMedia />
                            <FileItemContent>
                                <FileItemTitle />
                                <Show when={!isSmallRow() || control.status !== "done"}>
                                    <FileItemDescription />
                                </Show>
                            </FileItemContent>
                            <Show when={control.isRetryable || control.isRemovable}>
                                <FileItemActions />
                            </Show>
                        </Show>
                    </FileItemContext>
                );
            },
        },
    );
}

/** Render what opens a file, over all of it but its buttons, nothing for one that does not open. */
export function FileItemOpen(properties: {
    /** Its label, to open the file by default. */
    readonly "aria-label"?: string;
    /** The StyleX styles applied after its own. */
    readonly xstyle?: style.Styles;
}): JSX.Element {
    // label it by the file
    const item = useFileItem();
    const locale = useLocale();
    const part = {
        "data-slot": "file-item-open",
        get title() {
            return item.file.name;
        },
        get "aria-label"() {
            return properties["aria-label"] ?? locale.render(t`Open ${item.file.name}`);
        },
        onClick: (event: MouseEvent) => item.open(event),
    };
    const styled = () => style.attributes([state.focus, styles.open, properties.xstyle]);

    return (
        <Show when={item.isOpenable}>
            <Show when={item.source()} fallback={<button type="button" {...part} {...styled()} />}>
                {(source) => (
                    <a href={source()} target="_blank" rel="noreferrer" {...part} {...styled()} />
                )}
            </Show>
        </Show>
    );
}

/** Render the frame of a file's picture or kind, with how far it is sent. */
export function FileItemMedia(properties: FileItemElementProperties): JSX.Element {
    // frame the caller's own media, such as a video's first picture, else the file's
    const item = useFileItem();
    const isCard = (): boolean => item.layout === "card";
    const isSmallRow = (): boolean => item.size === "sm" && !isCard();

    return renderPart(
        "div",
        "file-item-media",
        properties,
        () => [
            styles.frame,
            isCard() ? styles.frameCard : styles.frameRow,
            isSmallRow() && styles.frameRowSmall,
            item.isFailed && styles.frameFailed,
        ],
        {
            get children() {
                return (
                    properties.children ?? (
                        <>
                            <FileItemPicture />
                            <FileItemProgress />
                        </>
                    )
                );
            },
        },
    );
}

/** Render a file's picture, or its icon for another file or one the browser cannot show. */
export function FileItemPicture(properties: {
    /** What stands for a file without a picture, its icon by default. */
    readonly fallback?: JSX.Element;
    /** The StyleX styles applied after its own. */
    readonly xstyle?: style.Styles;
}): JSX.Element {
    // show the icon once the picture fails
    const item = useFileItem();
    const [broken, setBroken] = createSignal<string>();
    const picture = (): string | undefined =>
        item.kind === "image" && item.source() !== broken() ? item.source() : undefined;

    return (
        <Show when={picture()} fallback={properties.fallback ?? <FileItemIcon />}>
            {(source) => (
                <img
                    src={source()}
                    alt=""
                    loading="lazy"
                    decoding="async"
                    onError={() => setBroken(source())}
                    {...style.attributes([
                        styles.picture,
                        item.status !== "done" && styles.waiting,
                        properties.xstyle,
                    ])}
                />
            )}
        </Show>
    );
}

/** Render the icon of a file's kind, or a warning for a failed one, nothing while it is sent. */
export function FileItemIcon(properties: Omit<IconProperties, "name">): JSX.Element {
    const item = useFileItem();

    return (
        <Show when={!item.isWaiting}>
            <Icon
                {...properties}
                icon={properties.icon ?? (item.isFailed ? warning : ICONS[item.kind])}
            />
        </Show>
    );
}

/** Render how far a file is sent as a ring, nothing for one that is not on its way. */
export function FileItemProgress(properties: {
    /** Its label, to send the file by default. */
    readonly "aria-label"?: string;
    /** The StyleX styles applied after its own. */
    readonly xstyle?: style.Styles;
}): JSX.Element {
    // turn until the first part arrived
    const item = useFileItem();
    const locale = useLocale();
    const isStarting = (): boolean => item.share === 0 && item.status === "sending";

    return (
        <Show when={item.isWaiting}>
            <svg
                viewBox="0 0 20 20"
                role="progressbar"
                data-slot="file-item-progress"
                aria-label={properties["aria-label"] ?? locale.render(t`Sending ${item.file.name}`)}
                aria-valuemin="0"
                aria-valuemax="100"
                aria-valuenow={isStarting() ? undefined : Math.round(item.share * 100)}
                {...style.attributes([
                    styles.ring,
                    item.layout === "card" && styles.ringCard,
                    isStarting() && styles.turning,
                    properties.xstyle,
                ])}
            >
                <circle cx="10" cy="10" r="8" stroke-width="2" {...style.attrs(styles.track)} />
                <circle
                    cx="10"
                    cy="10"
                    r="8"
                    stroke-width="2"
                    pathLength="1"
                    stroke-dasharray="1"
                    stroke-linecap="round"
                    {...style.attrs(arcs.arc(isStarting() ? STARTING_SHARE : item.share))}
                />
            </svg>
        </Show>
    );
}

/** Render a file's title and description, taking the row's free width. */
export function FileItemContent(properties: FileItemElementProperties): JSX.Element {
    const item = useFileItem();

    return renderPart("div", "file-item-content", properties, () => [
        styles.content,
        item.layout === "card" && styles.contentCard,
    ]);
}

/** Render a file's name, or another title. */
export function FileItemTitle(properties: FileItemElementProperties<HTMLSpanElement>): JSX.Element {
    const item = useFileItem();

    return renderPart("span", "file-item-title", properties, styles.title, {
        get title() {
            return item.file.name;
        },
        get children() {
            return properties.children ?? item.file.name;
        },
    });
}

/** Render what a file is, how far it is sent or why it failed, or another description. */
export function FileItemDescription(
    properties: FileItemElementProperties<HTMLSpanElement>,
): JSX.Element {
    // tell a failure as it happens
    const item = useFileItem();
    const locale = useLocale();

    return renderPart(
        "span",
        "file-item-description",
        properties,
        () => [text.xs, styles.description, item.isFailed && styles.error],
        {
            get role() {
                return item.isFailed ? "alert" : undefined;
            },
            get children() {
                return properties.children ?? describe(item, locale);
            },
        },
    );
}

/** Render a file's buttons, to send it again and to remove it by default. */
export function FileItemActions(properties: FileItemElementProperties): JSX.Element {
    const item = useFileItem();

    return renderPart(
        "div",
        "file-item-actions",
        properties,
        () => [styles.actions, item.layout === "card" && styles.actionsCard],
        {
            get children() {
                return (
                    properties.children ?? (
                        <>
                            <FileItemRetry />
                            <FileItemRemove />
                        </>
                    )
                );
            },
        },
    );
}

/** Render a button of a file, framed over a card's picture. */
export function FileItemAction(properties: ButtonProperties): JSX.Element {
    const item = useFileItem();
    const rest = omit(properties, "xstyle", "variant", "size");

    return (
        <Button
            data-slot="file-item-action"
            {...rest}
            variant={properties.variant ?? "ghost"}
            size={properties.size ?? "icon-sm"}
            xstyle={[styles.action, item.layout === "card" && styles.actionCard, properties.xstyle]}
        />
    );
}

/** Render the button that sends a failed file again, nothing for another. */
export function FileItemRetry(properties: ButtonProperties): JSX.Element {
    const item = useFileItem();
    const locale = useLocale();

    return (
        <Show when={item.isRetryable}>
            <FileItemAction
                data-slot="file-item-retry"
                aria-label={locale.render(t`Send ${item.file.name} again`)}
                {...mergeProperties({ onClick: () => item.retry() }, omit(properties, "children"))}
            >
                {properties.children ?? <Icon icon={arrowClockwise} />}
            </FileItemAction>
        </Show>
    );
}

/** Render the button that removes a file, nothing for one that stays. */
export function FileItemRemove(properties: ButtonProperties): JSX.Element {
    // move the focus to the file that takes its place, unless it moved elsewhere
    const item = useFileItem();
    const locale = useLocale();
    const remove = (event: MouseEvent): void => {
        // find the group before the file leaves it
        const removed =
            event.currentTarget instanceof Element
                ? event.currentTarget.closest("[data-slot=file-item]")
                : null;
        const group = removed?.parentElement;
        const at = [...(group?.children ?? [])].findIndex((each) => each === removed);
        item.remove();

        // take the button now at its place, else the one before
        queueMicrotask(() => {
            const left = group?.querySelectorAll<HTMLElement>("[data-slot=file-item-remove]");
            if (removed?.isConnected === false && document.activeElement === document.body) {
                (left?.[at] ?? left?.[at - 1])?.focus();
            }
        });
    };

    return (
        <Show when={item.isRemovable}>
            <FileItemAction
                data-slot="file-item-remove"
                aria-label={locale.render(t`Remove ${item.file.name}`)}
                {...mergeProperties({ onClick: remove }, omit(properties, "children"))}
            >
                {properties.children ?? <Icon icon={xIcon} />}
            </FileItemAction>
        </Show>
    );
}

/** The properties of a group of files, with its element's attributes. */
export interface FileItemGroupProperties extends FileItemElementProperties {
    /** How its files are laid out, in rows by default. */
    readonly layout?: FileItemLayout;
}

/** Render files as a list, in rows or as a grid of cards. */
export function FileItemGroup(properties: FileItemGroupProperties): JSX.Element {
    return (
        <FileItemGroupContext value={true}>
            {renderPart(
                "div",
                "file-item-group",
                omit(properties, "layout"),
                () => [styles.group, properties.layout === "card" && styles.cards],
                { role: "list" },
            )}
        </FileItemGroupContext>
    );
}

/** Write why a file could not be sent: in the app's words, as a refusal, or as its upload ended. */
function describeFailure({ error, reason }: FileItemControl, locale: Localization): string {
    if (typeof error === "string") {
        return error;
    } else if (error !== undefined) {
        return describeReason(error, locale);
    }

    return locale.render(reason === "connection" ? t`Connection lost` : t`Not sent`);
}

/** Write what a file is, how far it is sent, or why it failed. */
function describe(item: FileItemControl, locale: Localization): string {
    // write why it failed, or how far it is
    const { file, status } = item;
    if (item.isFailed) {
        return describeFailure(item, locale);
    } else if (item.isWaiting) {
        const word = locale.render(status === "paused" ? t`Paused` : t`Sending`);

        return item.share === 0
            ? word
            : `${word} · ${locale.number(item.share, { style: "percent" })}`;
    }

    // write its extension as people call its format, and its size
    const dot = file.name.lastIndexOf(".");
    const extension = dot > 0 ? file.name.slice(dot + 1) : "";
    const written = FileSize.write(file.size, locale.number);

    return extension === "" || extension.length > EXTENSION_LENGTH
        ? written
        : `${extension.toUpperCase()} · ${written}`;
}
