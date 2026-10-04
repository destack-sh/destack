import * as style from "@destack/style";
import { color, size, space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { createContext, merge, omit, useContext } from "solid-js";

/** The alignment of a message that sets none. */
const DEFAULTS: Required<Pick<MessageProperties, "align">> = { align: "start" };

/** The styles of a message and its elements. */
const styles = style.create({
    group: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
        minWidth: 0,
    },
    message: {
        position: "relative",
        display: "flex",
        gap: space[2],
        width: "100%",
        minWidth: 0,
    },
    end: {
        flexDirection: "row-reverse",
    },
    avatar: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        alignSelf: "flex-end",
        overflow: "hidden",
        width: "fit-content",
        minWidth: size[3],
        borderRadius: "50%",
        backgroundColor: color.muted,
    },
    content: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
        width: "100%",
        minWidth: 0,
        overflowWrap: "anywhere",
    },
    contentEnd: {
        alignItems: "flex-end",
    },
    caption: {
        display: "flex",
        alignItems: "center",
        maxWidth: "100%",
        minWidth: 0,
        paddingInline: space[3],
        color: color.mutedForeground,
        fontWeight: weight.medium,
    },
    captionEnd: {
        justifyContent: "flex-end",
    },
});

/** The side of the nearest message, which its content and captions follow. */
const MessageContext = createContext<() => MessageAlign>(() => "start");

/** The side a message sits on: the start for others' messages, the end for the reader's own. */
export type MessageAlign = "start" | "end";

/** The properties of an element of a message, the native element's attributes included. */
export type MessageElementProperties = Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** The properties of a message, the native element's attributes included. */
export interface MessageProperties extends MessageElementProperties {
    /** The side, start by default. */
    readonly align?: MessageAlign;
}

/** Render consecutive messages stacked together. */
export function MessageGroup(properties: MessageElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div data-slot="message-group" {...rest} {...style.attrs(styles.group, properties.style)} />
    );
}

/** Render a message in a conversation with its author's avatar beside its content. */
export function Message(properties: MessageProperties): JSX.Element {
    const message = merge(DEFAULTS, properties);
    const rest = omit(message, "align", "style");

    return (
        <MessageContext value={() => message.align}>
            <div
                data-slot="message"
                data-align={message.align}
                {...rest}
                {...style.attrs(
                    text.callout,
                    styles.message,
                    message.align === "end" && styles.end,
                    message.style,
                )}
            />
        </MessageContext>
    );
}

/** Render the avatar of a message's author at the message's foot. */
export function MessageAvatar(properties: MessageElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="message-avatar"
            {...rest}
            {...style.attrs(styles.avatar, properties.style)}
        />
    );
}

/** Render a message's bubbles, attachments and other content, aligned to its side. */
export function MessageContent(properties: MessageElementProperties): JSX.Element {
    const align = useContext(MessageContext);
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="message-content"
            {...rest}
            {...style.attrs(
                styles.content,
                align() === "end" && styles.contentEnd,
                properties.style,
            )}
        />
    );
}

/** Render the line above a message's content, such as its author's name. */
export function MessageHeader(properties: MessageElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="message-header"
            {...rest}
            {...style.attrs(text.caption, styles.caption, properties.style)}
        />
    );
}

/** Render the line below a message's content, such as when it was sent or read. */
export function MessageFooter(properties: MessageElementProperties): JSX.Element {
    const align = useContext(MessageContext);
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="message-footer"
            {...rest}
            {...style.attrs(
                text.caption,
                styles.caption,
                align() === "end" && styles.captionEnd,
                properties.style,
            )}
        />
    );
}
