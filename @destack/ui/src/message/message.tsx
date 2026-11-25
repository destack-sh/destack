import type { Clock } from "@destack/environment";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
} from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createSignal,
    createUniqueId,
    type JSX,
    merge,
    omit,
    onCleanup,
    untrack,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { CollectionBuilder } from "../collection/index.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";
import { Toolbar, type ToolbarProperties } from "../toolbar/index.ts";
import { messageMarker } from "./marker.stylex.ts";

/** The side of a message that sets none. */
const DEFAULTS = { align: "start" } as const;

/** Where a message's actions stand that set none. */
const ACTION_DEFAULTS = { placement: "float", autohide: "always" } as const;

/** The milliseconds a finger rests before a message shows its actions. */
const HOLD_DELAY = 500;

/** The pixels a finger may drift while it rests. */
const HOLD_TOLERANCE = 10;

/** The pixels a finger pulls a message before it lets go to answer it, a thumb's easy reach. */
const SWIPE_DISTANCE = 56;

/** The pixels from an edge of the screen the browser's own swipe starts in, as on iOS. */
const SCREEN_EDGE = 24;

/** The room of an avatar beside a message, as tall as two lines. */
const AVATAR = size[2];

/** The styles of a message and its parts. */
const styles = style.create({
    message: {
        position: "relative",
        display: "flex",
        alignItems: "flex-end",
        gap: space[2],
        minWidth: 0,
        width: "100%",
    },
    end: {
        flexDirection: "row-reverse",
    },
    // leave a pull to the side to the message, and one up or down to the page
    swipeable: {
        touchAction: "pan-y",
        transitionProperty: "translate",
        transitionDuration: { default: motion.durationMedium, [media.motionReduce]: "0s" },
        transitionTimingFunction: motion.easingStandard,
    },
    pulled: (distance: string) => ({
        translate: `${distance} 0`,
        transitionDuration: "0s",
    }),
    // leave a long press to the actions in place of the browser's own selection and menu
    actionable: {
        userSelect: { default: null, [media.pointerCoarse]: "none" },
        WebkitTouchCallout: { default: null, [media.pointerCoarse]: "none" },
    },
    avatar: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        width: AVATAR,
    },
    // stand beside the body, above a footer's line
    bottom: {
        translate: {
            default: null,
            [style.when.ancestor(":has([data-slot=message-footer])", messageMarker)]:
                `0 calc(-1lh - ${space[1]})`,
        },
    },
    top: {
        alignSelf: "flex-start",
    },
    // line up with the words of a bubble
    besideBubble: {
        paddingInline: {
            default: null,
            [style.when.ancestor(
                ":has([data-slot=bubble]:not([data-variant=ghost]))",
                messageMarker,
            )]: `calc(${space[3]} + ${stroke.border})`,
        },
    },
    content: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        gap: space[1],
        minWidth: 0,
    },
    contentEnd: {
        alignItems: "flex-end",
    },
    header: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "baseline",
        columnGap: space[2],
        minWidth: 0,
        color: color.mutedForeground,
    },
    author: {
        color: color.foreground,
        fontWeight: weight.medium,
        overflowWrap: "anywhere",
    },
    footer: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
        color: color.mutedForeground,
    },
    footerEnd: {
        justifyContent: "flex-end",
    },
    group: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        minWidth: 0,
        width: "100%",
    },
    actions: {
        opacity: {
            default: 0,
            ":focus-within": 1,
            ":has(button[aria-expanded=true])": 1,
            [style.when.ancestor(":hover", messageMarker)]: { default: 0, [media.hover]: 1 },
            [style.when.ancestor(":focus-visible", messageMarker)]: 1,
            [style.when.ancestor("[data-active]", messageMarker)]: 1,
        },
        pointerEvents: {
            default: "none",
            ":focus-within": "auto",
            ":has(button[aria-expanded=true])": "auto",
            [style.when.ancestor(":hover", messageMarker)]: {
                default: "none",
                [media.hover]: "auto",
            },
            [style.when.ancestor("[data-active]", messageMarker)]: "auto",
        },
        transitionProperty: "opacity",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    // stay on the last row of a conversation
    last: {
        opacity: {
            default: null,
            [style.when.ancestor(
                ":is([data-slot=message-scroller-item]:last-child *)",
                messageMarker,
            )]: 1,
        },
        pointerEvents: {
            default: null,
            [style.when.ancestor(
                ":is([data-slot=message-scroller-item]:last-child *)",
                messageMarker,
            )]: "auto",
        },
    },
    float: {
        position: "absolute",
        zIndex: 2,
        insetBlockStart: `calc(-1 * ${space[4]})`,
        insetInlineEnd: space[2],
        padding: `calc(${space[1]} / 2)`,
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.foreground,
        boxShadow: shadow.raised,
    },
    floatEnd: {
        insetInlineEnd: "auto",
        insetInlineStart: space[2],
    },
});

/** The side of a conversation a message stands at: the others' or one's own. */
export type MessageAlign = "start" | "end";

/** The side of the nearest message and its actions, which its parts share. */
class MessageControl {
    /** Read the side it stands at. */
    readonly align: () => MessageAlign;
    /** Whether a finger's long press shows its actions. */
    readonly isActive: Accessor<boolean>;
    /** How far a finger pulls it to the side, in pixels. */
    readonly pull: Accessor<number>;
    /** The actions it has. */
    readonly #actions: CollectionBuilder<string>;
    /** Read the direction the text runs in. */
    readonly #direction: () => "ltr" | "rtl";
    /** Read what a pull to the side does, undefined where it does nothing. */
    readonly #onSwipe: () => (() => void) | undefined;
    /** Set whether a long press shows its actions. */
    readonly #setActive: (isActive: boolean) => void;
    /** Set how far a finger pulls it. */
    readonly #setPull: (pull: number) => void;
    /** The clock a long press waits on. */
    readonly #clock: Pick<Clock, "setTimeout" | "clearTimeout">;
    /** The timer of a finger that rests. */
    #timer: number | undefined;
    /** Where the finger pressed. */
    #start: { readonly x: number; readonly y: number } | undefined;
    /** Whether the finger rested long enough, whose release presses nothing. */
    #isHeld = false;
    /** Stop listening for a press elsewhere. */
    #unwatch: () => void;

    /** Follow the side of a message's properties. */
    constructor(
        align: () => MessageAlign,
        onSwipe: () => (() => void) | undefined,
        direction: () => "ltr" | "rtl",
        clock: Pick<Clock, "setTimeout" | "clearTimeout">,
    ) {
        // start without a long press or a pull
        const [isActive, setActive] = createSignal(false, { ownedWrite: true });
        const [pull, setPull] = createSignal(0, { ownedWrite: true });
        this.align = align;
        this.isActive = isActive;
        this.pull = pull;
        this.#actions = new CollectionBuilder();
        this.#direction = direction;
        this.#onSwipe = onSwipe;
        this.#setActive = (next) => void setActive(next);
        this.#setPull = (next) => void setPull(next);
        this.#clock = clock;
        this.#unwatch = () => undefined;
        onCleanup(() => {
            // stop waiting and listening, the signals gone with it
            this.#clock.clearTimeout(this.#timer);
            this.#unwatch();
        });
    }

    /** Report whether it has actions a long press shows. */
    hasActions(): boolean {
        return this.#actions.items().length > 0;
    }

    /** Add its actions until they unmount. */
    addActions(id: string): void {
        this.#actions.add(id);
    }

    /** Read the handlers of a finger on the message, which leave its presses to what is inside. */
    attributes() {
        return {
            onTouchStart: (event: TouchEvent & { readonly currentTarget: HTMLElement }) =>
                this.#press(event),
            onTouchMove: (event: TouchEvent & { readonly currentTarget: HTMLElement }) =>
                this.#drift(event),
            onTouchEnd: (event: TouchEvent) => this.#drop(event),
            onTouchCancel: () => this.#lift(),
        };
    }

    /** Hide its actions. */
    leave(): void {
        this.#lift();
        this.#unwatch();
        this.#setActive(false);
    }

    /** Wait for a finger alone to rest, or to pull to the side. */
    #press(event: TouchEvent & { readonly currentTarget: HTMLElement }): void {
        // leave several fingers to the page
        const [touch, ...others] = event.touches;
        this.#isHeld = false;
        this.#lift();
        if (touch === undefined || others.length > 0) {
            return;
        }
        const message = event.currentTarget;
        this.#start = { x: touch.clientX, y: touch.clientY };
        if (this.hasActions()) {
            this.#timer = this.#clock.setTimeout(() => {
                // show its actions, until a press elsewhere
                this.#lift();
                this.#isHeld = true;
                this.#setActive(true);
                this.#leaveBy(message);
            }, HOLD_DELAY);
        }
    }

    /** Follow a finger that drifts: to the side it pulls, else it scrolls. */
    #drift(event: TouchEvent & { readonly currentTarget: HTMLElement }): void {
        // measure how far it is, forward in the direction the text runs in
        const [touch] = event.touches;
        const start = this.#start;
        if (touch === undefined || start === undefined) {
            return;
        }
        const x = touch.clientX - start.x;
        const y = touch.clientY - start.y;
        const forward = this.#direction() === "rtl" ? -x : x;
        const width = event.currentTarget.ownerDocument.documentElement.clientWidth;
        const isEdge = start.x < SCREEN_EDGE || start.x > width - SCREEN_EDGE;
        const isAside = forward > HOLD_TOLERANCE && forward > 2 * Math.abs(y);

        // pull it, let go of one that scrolls, or wait on
        if (this.#onSwipe() !== undefined && !isEdge && (isAside || untrack(this.pull) > 0)) {
            this.#clock.clearTimeout(this.#timer);
            this.#setPull(Math.min(Math.max(0, forward), SWIPE_DISTANCE));
        } else if (Math.hypot(x, y) > HOLD_TOLERANCE) {
            this.#lift();
        }
    }

    /** Answer a message pulled far enough, and press nothing after a long press or a pull. */
    #drop(event: TouchEvent): void {
        // read how far it was pulled before letting go
        const pull = untrack(this.pull);
        const isFar = pull >= SWIPE_DISTANCE;
        if (this.#isHeld || pull > 0) {
            event.preventDefault();
        }
        this.#lift();
        if (isFar) {
            this.#onSwipe()?.();
        }
    }

    /** Let go of the finger. */
    #lift(): void {
        // forget where it pressed
        this.#clock.clearTimeout(this.#timer);
        this.#timer = undefined;
        this.#start = undefined;
        this.#setPull(0);
    }

    /** Hide its actions by a press elsewhere. */
    #leaveBy(message: HTMLElement): void {
        // keep them while a press lands inside the message
        this.#unwatch();
        const { ownerDocument } = message;
        const leave = (event: PointerEvent): void => {
            const isInside = event.target instanceof Node && message.contains(event.target);
            if (!isInside) {
                this.leave();
            }
        };
        ownerDocument.addEventListener("pointerdown", leave, { capture: true });
        this.#unwatch = () => {
            ownerDocument.removeEventListener("pointerdown", leave, { capture: true });
            this.#unwatch = () => undefined;
        };
    }
}

/** The nearest message, null outside one. */
const MessageContext = createContext<MessageControl | null>(null);

/** The properties of an element of a message, the native element's attributes included. */
export type MessageElementProperties<Target extends HTMLElement = HTMLDivElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** The properties of a message. */
export type MessageProperties = MessageElementProperties & {
    /** The side it stands at, the start by default. */
    readonly align?: MessageAlign;
    /** Handle a finger that pulls it to the side, such as to answer it, off by default. */
    readonly onSwipe?: () => void;
};

/** Render one message: its avatar beside its header, body and footer. */
export function Message(properties: MessageProperties): JSX.Element {
    // show its actions on a long press by a finger, until a press elsewhere
    const message = merge(DEFAULTS, properties);
    const rest = omit(properties, "align", "onSwipe");
    const locale = useLocale();
    const control = new MessageControl(
        () => message.align,
        () => properties.onSwipe,
        () => locale.direction,
        useEnvironment().clock,
    );
    const distance = (): string =>
        `${String(locale.direction === "rtl" ? -control.pull() : control.pull())}px`;

    return (
        <MessageContext value={control}>
            {renderPart(
                "div",
                "message",
                rest,
                () => [
                    messageMarker,
                    text.sm,
                    styles.message,
                    message.align === "end" && styles.end,
                    control.hasActions() && styles.actionable,
                    properties.onSwipe !== undefined && styles.swipeable,
                    control.pull() > 0 && styles.pulled(distance()),
                ],
                mergeProperties(control.attributes(), {
                    get "data-align"() {
                        return message.align;
                    },
                    get "data-active"() {
                        return control.isActive() ? "" : undefined;
                    },
                    get "data-swiping"() {
                        return control.pull() > 0 ? "" : undefined;
                    },
                }),
            )}
        </MessageContext>
    );
}

/** The properties of a message's avatar. */
export type MessageAvatarProperties = MessageElementProperties & {
    /** Where it stands beside the message, at its end by default, as under a bubble's tail. */
    readonly position?: "top" | "bottom";
};

/** Render the room of an avatar beside a message, empty for all but one of a group. */
export function MessageAvatar(properties: MessageAvatarProperties): JSX.Element {
    const rest = omit(properties, "position");

    return renderPart("div", "message-avatar", rest, () => [
        text.xs,
        styles.avatar,
        properties.position === "top" ? styles.top : styles.bottom,
    ]);
}

/** Render the header, the body and the footer of a message below each other. */
export function MessageContent(properties: MessageElementProperties): JSX.Element {
    const align = useMessageAlign();

    return renderPart("div", "message-content", properties, () => [
        styles.content,
        align() === "end" && styles.contentEnd,
    ]);
}

/** Render what stands above a message, such as who wrote it and when. */
export function MessageHeader(properties: MessageElementProperties): JSX.Element {
    return renderPart("div", "message-header", properties, [
        text.xs,
        styles.header,
        styles.besideBubble,
    ]);
}

/** Render the name of who wrote a message, in its header. */
export function MessageAuthor(properties: MessageElementProperties<HTMLSpanElement>): JSX.Element {
    return renderPart("span", "message-author", properties, [text.sm, styles.author]);
}

/** Render what stands below a message, such as whether it was read, or its actions. */
export function MessageFooter(properties: MessageElementProperties): JSX.Element {
    const align = useMessageAlign();

    return renderPart("div", "message-footer", properties, () => [
        text.xs,
        styles.footer,
        styles.besideBubble,
        align() === "end" && styles.footerEnd,
    ]);
}

/** The properties of a message's actions. */
export type MessageToolbarProperties = ToolbarProperties & {
    /** Whether they float over the message's corner, or stand in its footer. */
    readonly placement?: "float" | "inline";
    /** Whether they hide until the message is pointed at: always by default, or not its last. */
    readonly autohide?: "always" | "not-last" | "never";
};

/** Render a message's actions, shown while it is pointed at, focused or pressed long. */
export function MessageToolbar(properties: MessageToolbarProperties): JSX.Element {
    // offer them to a long press
    const actions = merge(ACTION_DEFAULTS, properties);
    const align = useMessageAlign();
    useContext(MessageContext)?.addActions(createUniqueId());
    const rest = omit(properties, "placement", "autohide", "xstyle");

    return (
        <Toolbar
            data-slot="message-toolbar"
            {...rest}
            xstyle={[
                actions.autohide !== "never" && styles.actions,
                actions.autohide === "not-last" && styles.last,
                actions.placement === "float" && styles.float,
                actions.placement === "float" && align() === "end" && styles.floatEnd,
                properties.xstyle,
            ]}
        />
    );
}

/** Read the side of the nearest message, the start outside one. */
export function useMessageAlign(): () => MessageAlign {
    const control = useContext(MessageContext);

    return () => control?.align() ?? DEFAULTS.align;
}

/** Render the messages of one person in a row. */
export function MessageGroup(properties: MessageElementProperties): JSX.Element {
    return renderPart("div", "message-group", properties, styles.group);
}
