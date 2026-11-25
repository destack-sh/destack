import arrowDownIcon from "@destack/icon/phosphor/arrow-down";
import arrowUpIcon from "@destack/icon/phosphor/arrow-up";
import { plural, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { color, motion, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import { type JSX, omit, onSettled, Show, useContext, useLocale } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";
import {
    MESSAGE_ID,
    MessageScrollerContext,
    MessageScrollerControl,
    type MessageScrollerOptions,
    SCROLL_ANCHOR,
    useMessageScroller,
} from "./control.ts";

/** The height a row is taken to have until it is drawn. */
const ROW_HEIGHT = "auto 10rem";

/** The styles of a message scroller and its parts. */
const styles = style.create({
    frame: {
        position: "relative",
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
        minWidth: 0,
        height: "100%",
    },
    viewport: {
        flexGrow: 1,
        minHeight: 0,
        overflowX: "hidden",
        overflowY: "auto",
        overscrollBehavior: "contain",
        // bring a focused row as far in as the content's padding
        scrollPaddingBlock: space[4],
        // keep the reader's place by the rows, as not every browser anchors them
        overflowAnchor: "none",
        scrollbarWidth: "thin",
        scrollbarGutter: "stable",
        // fade what is below the view
        maskImage: {
            default: null,
            ":is([data-scrollable~=end])": {
                default: `linear-gradient(to bottom, black calc(100% - ${space[6]}), transparent)`,
                [media.forcedColors]: "none",
            },
        },
        // hide the bar while the view moves by itself
        scrollbarColor: {
            default: `${color.border} transparent`,
            ":is([data-autoscrolling])": "transparent transparent",
        },
    },
    content: {
        display: "flex",
        flexDirection: "column",
        gap: space[6],
        minWidth: 0,
        padding: space[4],
        // stay at the top of a view taller than the rows, also while it waits to open
        marginBlockEnd: "auto",
    },
    item: {
        flexShrink: 0,
        minWidth: 0,
        // draw it only near the view, whole where something of it stands over its edges
        contentVisibility: {
            default: "auto",
            ":hover": { default: null, [media.hover]: "visible" },
            ":focus-within": "visible",
            ":has([data-slot=bubble-overlay], [data-slot=message][data-active])": "visible",
        },
        containIntrinsicSize: ROW_HEIGHT,
        borderRadius: radius[3],
    },
    spacer: {
        flexShrink: 0,
        height: 0,
    },
    button: {
        position: "absolute",
        zIndex: 3,
        insetInlineStart: "50%",
        translate: "-50% 0",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius.full,
        backgroundColor: color.popover,
        boxShadow: shadow.raised,
        transitionProperty: "opacity, transform, translate",
        transitionDuration: { default: motion.durationShort, [media.motionReduce]: "0s" },
        transitionTimingFunction: motion.easingStandard,
    },
    end: { insetBlockEnd: space[4] },
    start: { insetBlockStart: space[4] },
    hidden: {
        opacity: 0,
        pointerEvents: "none",
        transform: "scale(0.95)",
    },
    hiddenEnd: { translate: "-50% 100%" },
    hiddenStart: { translate: "-50% -100%" },
});

/** The look of a conversation until it opens, which a page without JavaScript keeps. */
const pending = style.create({
    start: {},
    // show the end from the first paint
    end: {
        display: "flex",
        flexDirection: "column-reverse",
    },
    // hide the jump to the turn
    "last-anchor": {
        visibility: "hidden",
    },
});

/** The properties of a provider of a conversation. */
export interface MessageScrollerProviderProperties extends MessageScrollerOptions {
    /** The conversation, and controls of it outside its frame. */
    readonly children?: JSX.Element;
}

/** Share where a reader is in a conversation with its parts and with controls outside its frame. */
export function MessageScrollerProvider(
    properties: MessageScrollerProviderProperties,
): JSX.Element {
    const control = new MessageScrollerControl(properties);

    return <MessageScrollerContext value={control}>{properties.children}</MessageScrollerContext>;
}

/** The properties of an element of a conversation, the native element's attributes included. */
export type MessageScrollerElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of a conversation. */
export type MessageScrollerProperties = MessageScrollerElementProperties & MessageScrollerOptions;

/** Render a conversation that keeps the reader's place around its parts. */
export function MessageScroller(properties: MessageScrollerProperties): JSX.Element {
    // share a conversation of its own where no provider does
    const provided = useContext(MessageScrollerContext);
    const control = provided ?? new MessageScrollerControl(properties);
    const rest = omit(
        properties,
        "autoScroll",
        "defaultScrollPosition",
        "scrollEdgeThreshold",
        "scrollMargin",
        "scrollPreviousItemPeek",
    );

    return (
        <MessageScrollerContext value={control}>
            {renderPart("div", "message-scroller", rest, styles.frame, attributesOf(control))}
        </MessageScrollerContext>
    );
}

/** Render the part of a conversation that scrolls, around its content. */
export function MessageScrollerViewport(properties: MessageScrollerElementProperties): JSX.Element {
    // say it scrolls, and which way it still does
    const control = useMessageScroller();
    const locale = useLocale();

    return renderPart(
        "div",
        "message-scroller-viewport",
        properties,
        () => [state.focusInset, styles.viewport, control.isPending() && pending[control.position]],
        mergeProperties(attributesOf(control), {
            role: "region" as const,
            tabindex: 0,
            get "aria-label"() {
                return locale.render(t`Messages`);
            },
        }),
    );
}

/** Render the rows of a conversation as a log, each any element, with the room below them. */
export function MessageScrollerContent(properties: MessageScrollerElementProperties): JSX.Element {
    // follow the reader and the rows
    const control = useMessageScroller();
    let content: HTMLElement | undefined;
    let spacer: HTMLElement | undefined;
    onSettled(() => {
        const viewport = content?.closest<HTMLElement>("[data-slot=message-scroller-viewport]");
        if (content === undefined || spacer === undefined || viewport == null) {
            throw new TypeError("the content of a message scroller needs a viewport around it");
        }

        return control.observe(viewport, content, spacer);
    });

    return (
        <>
            {renderPart(
                "div",
                "message-scroller-content",
                properties,
                styles.content,
                mergeProperties(
                    {
                        role: "log" as const,
                        "aria-relevant": "additions" as const,
                        onKeyDown: (event: KeyboardEvent) => step(event, control),
                    },
                    { ref: (element: HTMLElement) => (content = element) },
                ),
            )}
            <div
                aria-hidden="true"
                data-slot="message-scroller-spacer"
                ref={(element) => (spacer = element)}
                {...style.attrs(styles.spacer)}
            />
        </>
    );
}

/** The properties of a row of a conversation. */
export type MessageScrollerItemProperties = MessageScrollerElementProperties & {
    /** The id a reader jumps to it by. */
    readonly messageId?: string;
    /** Whether it starts a turn, which comes near the top as it is added. */
    readonly scrollAnchor?: boolean;
};

/** Render a row of a conversation, drawn only near the view. */
export function MessageScrollerItem(properties: MessageScrollerItemProperties): JSX.Element {
    const rest = omit(properties, "messageId", "scrollAnchor");

    return renderPart("div", "message-scroller-item", rest, [state.focusInset, styles.item], {
        tabindex: -1,
        get [MESSAGE_ID]() {
            return properties.messageId;
        },
        get [SCROLL_ANCHOR]() {
            return properties.scrollAnchor === true ? "" : undefined;
        },
    });
}

/** The properties of the button that scrolls a conversation to one of its ends. */
export type MessageScrollerButtonProperties = Omit<ButtonProperties, "children"> & {
    /** The end it scrolls to, the latest message by default. */
    readonly direction?: "start" | "end";
    /** How it moves there, smoothly by default. */
    readonly behavior?: ScrollBehavior;
    /** Render its content for the count of rows unseen, an arrow and the count by default. */
    readonly children?: (unseen: number) => JSX.Element;
};

/** Render the button that scrolls a conversation to an end, out of reach while it is there. */
export function MessageScrollerButton(properties: MessageScrollerButtonProperties): JSX.Element {
    // say how many messages came meanwhile
    const control = useMessageScroller();
    const locale = useLocale();
    const isStart = (): boolean => properties.direction === "start";
    const isIdle = (): boolean =>
        isStart() ? !control.scrollable().start : !control.scrollable().end;
    const unseen = (): number => (isStart() ? 0 : control.unseen());
    const label = (): string => {
        // count what came while the reader was elsewhere
        if (isStart()) {
            return locale.render(t`Jump to first message`);
        }

        return unseen() === 0
            ? locale.render(t`Jump to latest message`)
            : locale.render(
                  t`${plural(unseen(), { one: "# new message", other: "# new messages" })}`,
              );
    };
    const rest = omit(properties, "direction", "behavior", "children", "xstyle");

    return (
        <Button
            data-slot="message-scroller-button"
            data-direction={isStart() ? "start" : "end"}
            data-active={isIdle() ? "false" : "true"}
            variant="outline"
            size={unseen() > 0 ? "sm" : "icon-sm"}
            aria-label={label()}
            inert={isIdle()}
            tabindex={isIdle() ? -1 : undefined}
            {...mergeProperties(
                {
                    onClick: () => {
                        // go to the end it stands for
                        const { behavior = "smooth" } = properties;
                        if (isStart()) {
                            control.scrollToStart({ behavior });
                        } else {
                            control.scrollToEnd({ behavior });
                        }
                    },
                },
                rest,
            )}
            xstyle={[
                styles.button,
                isStart() ? styles.start : styles.end,
                isIdle() && [styles.hidden, isStart() ? styles.hiddenStart : styles.hiddenEnd],
                properties.xstyle,
            ]}
        >
            {properties.children?.(unseen()) ?? (
                <>
                    <Icon icon={isStart() ? arrowUpIcon : arrowDownIcon} />
                    <Show when={unseen() > 0}>{label()}</Show>
                </>
            )}
        </Button>
    );
}

/** Read the state of a conversation its frame and its viewport are styled by. */
function attributesOf(control: MessageScrollerControl) {
    return {
        get "data-scrollable"() {
            // list the ends it scrolls toward
            const edges = [
                ...(control.scrollable().start ? ["start"] : []),
                ...(control.scrollable().end ? ["end"] : []),
            ];

            return edges.length === 0 ? undefined : edges.join(" ");
        },
        get "data-autoscrolling"() {
            return control.isAutoScrolling() ? "" : undefined;
        },
        get "data-pending-scroll"() {
            return control.isPending() ? "" : undefined;
        },
    };
}

/** Move between rows by the arrow keys while a row has the focus. */
function step(event: KeyboardEvent, control: MessageScrollerControl): void {
    // leave keys inside a row to what takes them there
    const { target } = event;
    const row = target instanceof HTMLElement && target.hasAttribute(MESSAGE_ID) ? target : null;
    const rows = [...(row?.parentElement?.children ?? [])].filter(
        (each): each is HTMLElement => each instanceof HTMLElement && each.hasAttribute(MESSAGE_ID),
    );
    const next = {
        ArrowDown: rows[rows.findIndex((each) => each === row) + 1],
        ArrowUp: rows[rows.findIndex((each) => each === row) - 1],
        Home: rows[0],
        End: rows.at(-1),
    }[event.key];
    if (next === undefined || row === null || event.defaultPrevented) {
        return;
    }

    // go to an end whole, and as far as a row else
    event.preventDefault();
    const isEnd = event.key === "Home" || event.key === "End";
    next.focus({ preventScroll: isEnd });
    if (event.key === "Home") {
        control.scrollToStart();
    } else if (event.key === "End") {
        control.scrollToEnd();
    }
}
