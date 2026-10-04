import { Icon } from "@destack/icon";
import { t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";
import * as style from "@destack/style";
import { color, motion, space } from "@destack/theme/tokens.stylex";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createSignal,
    merge,
    omit,
    onSettled,
    useContext,
    type Accessor,
    type Setter,
} from "solid-js";
import { Button, type ButtonProperties } from "../button/index.ts";

/** The distance from an edge, in pixels, that still counts as at the edge. */
const EDGE_TOLERANCE = 8;

/** The direction of a scroll button that sets none. */
const BUTTON_DEFAULTS: Required<
    Pick<MessageScrollerButtonProperties, "direction" | "variant" | "size">
> = {
    direction: "end",
    variant: "secondary",
    size: "icon-sm",
};

/** The styles of a message scroller and its elements. */
const styles = style.create({
    scroller: {
        position: "relative",
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        minHeight: 0,
        overflow: "hidden",
    },
    viewport: {
        width: "100%",
        height: "100%",
        minWidth: 0,
        minHeight: 0,
        overflowY: "auto",
        overflowAnchor: "auto",
        overscrollBehavior: "contain",
        scrollbarGutter: "stable",
        scrollbarWidth: "thin",
    },
    content: {
        display: "flex",
        flexDirection: "column",
        gap: space[7],
        minHeight: "100%",
    },
    item: {
        flexShrink: 0,
        minWidth: 0,
        contentVisibility: "auto",
        containIntrinsicSize: "auto 10rem",
    },
    button: {
        position: "absolute",
        insetInlineStart: "50%",
        borderColor: color.border,
        transform: "translateX(-50%)",
        transitionProperty: "opacity, translate, scale",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingEmphasised,
    },
    end: { bottom: space[4] },
    start: { top: space[4] },
    hidden: {
        opacity: 0,
        scale: "0.95",
        pointerEvents: "none",
    },
    icon: {
        display: "inline-flex",
    },
    upward: {
        rotate: "180deg",
    },
});

/** The scroll state of the nearest message scroller, null outside one. */
const MessageScrollerContext = createContext<MessageScrollerControl | null>(null);

/** The side of a conversation a scroll button moves to: its newest messages or its oldest. */
export type MessageScrollerDirection = "start" | "end";

/** The viewport of a message scroller, whether it rests at either edge, and the moves to them. */
export class MessageScrollerControl {
    /** Whether the viewport shows the oldest messages. */
    readonly isAtStart: Accessor<boolean>;
    /** Whether the viewport shows the newest messages, which keeps it following new ones. */
    readonly isAtEnd: Accessor<boolean>;
    /** The scrolling element, once mounted. */
    #viewport: HTMLElement | undefined;
    /** Replace whether the viewport shows the oldest messages. */
    readonly #setAtStart: Setter<boolean>;
    /** Replace whether the viewport shows the newest messages. */
    readonly #setAtEnd: Setter<boolean>;

    /** Create the state of a scroller resting at its newest messages. */
    constructor() {
        // start at the newest messages
        const [isAtStart, setAtStart] = createSignal(false);
        const [isAtEnd, setAtEnd] = createSignal(true);
        this.isAtStart = isAtStart;
        this.isAtEnd = isAtEnd;
        this.#setAtStart = setAtStart;
        this.#setAtEnd = setAtEnd;
    }

    /** Take the scrolling element and follow its content, starting at the newest messages. */
    attach(viewport: HTMLElement, content: HTMLElement): () => void {
        // start at the newest messages
        this.#viewport = viewport;
        viewport.scrollTop = viewport.scrollHeight;
        this.measure();

        // keep following the newest messages as content grows while the viewport rests at them
        const observer = new ResizeObserver(() => {
            if (this.isAtEnd()) {
                viewport.scrollTop = viewport.scrollHeight;
            }
            this.measure();
        });
        observer.observe(content);

        return () => observer.disconnect();
    }

    /** Read whether the viewport rests at either edge. */
    measure(): void {
        // wait for the viewport to mount
        const viewport = this.#viewport;
        if (viewport === undefined) {
            return;
        }

        // compare the scroll position with both edges
        const end = viewport.scrollHeight - viewport.clientHeight;
        this.#setAtStart(viewport.scrollTop <= EDGE_TOLERANCE);
        this.#setAtEnd(end - viewport.scrollTop <= EDGE_TOLERANCE);
    }

    /** Scroll smoothly to the oldest or newest messages. */
    scrollTo(direction: MessageScrollerDirection): void {
        this.#viewport?.scrollTo({
            top: direction === "end" ? this.#viewport.scrollHeight : 0,
            behavior: "smooth",
        });
    }
}

/** The properties of an element of a message scroller, the native element's attributes included. */
export type MessageScrollerElementProperties = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** The properties of a message scroller's button, a secondary small icon button by default. */
export interface MessageScrollerButtonProperties extends ButtonProperties {
    /** The side the button scrolls to, the newest messages by default. */
    readonly direction?: MessageScrollerDirection;
}

/** Render a conversation's scrolling frame, which follows new messages while it rests at the newest. */
export function MessageScroller(properties: MessageScrollerElementProperties): JSX.Element {
    const control = new MessageScrollerControl();
    const rest = omit(properties, "style");

    return (
        <MessageScrollerContext value={control}>
            <div
                data-slot="message-scroller"
                {...rest}
                {...style.attrs(styles.scroller, properties.style)}
            />
        </MessageScrollerContext>
    );
}

/** Render the element that scrolls a conversation, holding its content. */
export function MessageScrollerViewport(properties: MessageScrollerElementProperties): JSX.Element {
    // read the scroller and keep the viewport and its content for it
    const control = useMessageScroller();
    const rest = omit(properties, "style", "children");
    let viewport: HTMLDivElement | undefined;
    let content: HTMLDivElement | undefined;

    // follow the content once mounted
    onSettled(() =>
        viewport === undefined || content === undefined
            ? undefined
            : control.attach(viewport, content),
    );

    return (
        <div
            data-slot="message-scroller-viewport"
            tabindex={0}
            {...rest}
            ref={(element) => (viewport = element)}
            onScroll={() => control.measure()}
            {...style.attrs(styles.viewport, properties.style)}
        >
            <div ref={(element) => (content = element)} {...style.attrs(styles.content)}>
                {properties.children}
            </div>
        </div>
    );
}

/** Render one entry of a conversation, which the browser skips rendering while it is off screen. */
export function MessageScrollerItem(properties: MessageScrollerElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="message-scroller-item"
            {...rest}
            {...style.attrs(styles.item, properties.style)}
        />
    );
}

/** Render a button that scrolls to the newest or oldest messages, shown only while the viewport rests away from them. */
export function MessageScrollerButton(properties: MessageScrollerButtonProperties): JSX.Element {
    // read the scroller and name the button by its direction
    const control = useMessageScroller();
    const locale = useLocale();
    const button = merge(BUTTON_DEFAULTS, properties);
    const rest = omit(button, "direction", "style", "children");
    const isAtEdge = (): boolean =>
        button.direction === "end" ? control.isAtEnd() : control.isAtStart();

    return (
        <Button
            data-slot="message-scroller-button"
            data-direction={button.direction}
            aria-label={
                button.direction === "end"
                    ? locale.render(t`Scroll to newest`)
                    : locale.render(t`Scroll to oldest`)
            }
            inert={isAtEdge()}
            {...rest}
            onClick={() => control.scrollTo(button.direction)}
            style={[
                styles.button,
                styles[button.direction],
                isAtEdge() && styles.hidden,
                button.style,
            ]}
        >
            {button.children ?? (
                <span {...style.attrs(styles.icon, button.direction === "start" && styles.upward)}>
                    <Icon name="arrow-down" />
                </span>
            )}
        </Button>
    );
}

/** Read the scroll state of the nearest message scroller. */
export function useMessageScroller(): MessageScrollerControl {
    const control = useContext(MessageScrollerContext);
    if (control === null) {
        throw new TypeError("message scroller elements need a message scroller around them");
    }

    return control;
}
