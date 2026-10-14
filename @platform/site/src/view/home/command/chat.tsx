import { font } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { createSignal, type JSX } from "@destack/view";

import { browserStyles } from "../figure/browser";
import { Favicon, Glyph } from "../figure/glyph";
import { assistant, chat, chatgpt, claude, gemini } from "./chat.stylex";
import { type Item, turns } from "./scene";

/** The sans the assistants' chrome and your messages set in. */
const SANS = 'ui-sans-serif, -apple-system, "Segoe UI", system-ui, sans-serif';

/** The servers the assistant's tools run on, by their mark: your space, or a vendor's connector. */
const serverNames: Readonly<Record<string, string>> = {
    destack: "Destack",
    linear: "Linear",
    notion: "Notion",
    slack: "Slack",
    googlesheets: "Google Sheets",
    gmail: "Gmail",
};

/** The assistants the chat takes the look of, in turn. */
export const assistants = ["Claude", "ChatGPT", "Gemini"] as const;

/** One assistant the chat can look like. */
export type Assistant = (typeof assistants)[number];

/** Draw an assistant's desktop app a turn at a time: through connectors, or with your space. */
export function AgentChat(properties: {
    isOpen: boolean;
    turn: number;
    isLanded: boolean;
    assistant: Assistant;
}) {
    // show the turn before the open one as it ended, then the open turn as it stands
    const items = (): readonly Item[] => {
        const earlier = (properties.turn - 1 + turns.length) % turns.length;

        return [...itemsOf(earlier, true), ...itemsOf(properties.turn, properties.isLanded)];
    };

    // read a turn's items in the state the stack is in, your space's results once they land
    const itemsOf = (index: number, isLanded: boolean): readonly Item[] => {
        // read the turn, and its side in the state the stack is in
        const turn = turns[index];
        if (turn === undefined) {
            return [];
        } else if (!properties.isOpen) {
            return turn.stacked;
        }

        // replace a pending approval with its outcome once it lands
        const before = isLanded
            ? turn.destacked.before.filter((item) => item.kind !== "approval")
            : turn.destacked.before;

        return [...before, ...(isLanded ? turn.destacked.after : [])];
    };

    // tell each assistant's chrome apart, so the shown one fades in over the rest
    const isShown = (candidate: Assistant) => candidate === properties.assistant;

    return (
        <div
            data-component="AgentChat"
            data-assistant={properties.assistant}
            {...style.attrs(browserStyles.browser, styles.window, themes[properties.assistant])}
        >
            {/* title bar, one per assistant */}
            <div {...style.attrs(styles.stack)}>
                <Layer isShown={isShown("Claude")}>
                    <span {...style.attrs(styles.bar)}>
                        <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                        <Glyph name="sidebar" size={15} />
                        <span {...style.attrs(styles.conversation)}>
                            Launch day
                            <Glyph name="caret" size={11} />
                        </span>
                        <span {...style.attrs(styles.barEnd)}>
                            <span {...style.attrs(styles.share)}>Share</span>
                        </span>
                    </span>
                </Layer>
                <Layer isShown={isShown("ChatGPT")}>
                    <span {...style.attrs(styles.bar)}>
                        <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                        <Glyph name="sidebar" size={15} />
                        <Glyph name="compose" size={15} />
                        <span {...style.attrs(styles.barEnd)}>
                            <span {...style.attrs(styles.share, styles.chatgptShare)}>
                                <Glyph name="share" size={13} />
                                Share
                            </span>
                            <Glyph name="more" size={15} />
                        </span>
                    </span>
                </Layer>
                <Layer isShown={isShown("Gemini")}>
                    <span {...style.attrs(styles.bar)}>
                        <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                        <Glyph name="hamburger" size={16} />
                        <span {...style.attrs(styles.barEnd)}>
                            <Glyph name="grid" size={15} />
                            <span {...style.attrs(styles.avatar)}>F</span>
                        </span>
                    </span>
                </Layer>
            </div>

            {/* the conversation, newest at the bottom, settling into each assistant's type */}
            <div
                {...style.attrs(
                    styles.messages,
                    assistants.indexOf(properties.assistant) % 2 === 0
                        ? styles.settleEven
                        : styles.settleOdd,
                )}
            >
                {items().map((item) => (
                    <ItemView item={item} assistant={properties.assistant} />
                ))}

                {/* each assistant's mark or actions under its last reply */}
                <span {...style.attrs(styles.stack, styles.footer)}>
                    <Layer isShown={isShown("Claude")}>
                        <Favicon icon="claude" tint={assistant.clay} size={18} />
                    </Layer>
                    <Layer isShown={isShown("ChatGPT")}>
                        <span {...style.attrs(styles.actions)}>
                            <Glyph name="copy" size={14} />
                            <Glyph name="like" size={14} />
                            <Glyph name="dislike" size={14} />
                            <Glyph name="reload" size={14} />
                        </span>
                    </Layer>
                    <Layer isShown={isShown("Gemini")}>
                        <span {...style.attrs(styles.actions)}>
                            <Glyph name="like" size={14} />
                            <Glyph name="dislike" size={14} />
                            <Glyph name="share" size={14} />
                            <Glyph name="more" size={14} />
                        </span>
                    </Layer>
                </span>
            </div>

            {/* composer, one per assistant */}
            <div {...style.attrs(styles.stack, styles.composers)}>
                <Layer isShown={isShown("Claude")}>
                    <span {...style.attrs(styles.composer, styles.claudeComposer)}>
                        <span {...style.attrs(styles.placeholder)}>Reply to Claude…</span>
                        <span {...style.attrs(styles.composerRow)}>
                            <span {...style.attrs(styles.claudePlus)}>
                                <Glyph name="plus" size={14} />
                            </span>
                            <span {...style.attrs(styles.claudePlus)}>
                                <Glyph name="sliders" size={14} />
                            </span>
                            <Connectors isOpen={properties.isOpen} />
                            <span {...style.attrs(styles.hint, styles.end)}>
                                Opus 5.5
                                <Glyph name="caret" size={10} />
                            </span>
                            <span {...style.attrs(styles.claudeSend, styles.idle)}>
                                <Glyph name="up" size={14} weight={2.5} />
                            </span>
                        </span>
                    </span>
                </Layer>
                <Layer isShown={isShown("ChatGPT")}>
                    <span {...style.attrs(styles.composer, styles.chatgptComposer)}>
                        <span {...style.attrs(styles.placeholder)}>Ask anything</span>
                        <span {...style.attrs(styles.composerRow)}>
                            <Glyph name="plus" size={18} />
                            <Connectors isOpen={properties.isOpen} />
                            <span {...style.attrs(styles.end)}>
                                <Glyph name="microphone" size={16} />
                            </span>
                            <span {...style.attrs(styles.chatgptSend)}>
                                <Glyph name="waveform" size={15} weight={2.25} />
                            </span>
                        </span>
                    </span>
                </Layer>
                <Layer isShown={isShown("Gemini")}>
                    <span {...style.attrs(styles.composer, styles.geminiComposer)}>
                        <span {...style.attrs(styles.placeholder)}>Ask Gemini</span>
                        <span {...style.attrs(styles.composerRow)}>
                            <Glyph name="plus" size={16} />
                            <span {...style.attrs(styles.geminiTools)}>
                                <Glyph name="sliders" size={14} />
                                Tools
                            </span>
                            <Connectors isOpen={properties.isOpen} />
                            <span {...style.attrs(styles.end)}>
                                <Glyph name="microphone" size={18} />
                            </span>
                        </span>
                    </span>
                </Layer>
            </div>
        </div>
    );
}

/** The connectors the assistant reaches your apps through, by mark and colour. */
const vendors = [
    ["linear", "#5e6ad2"],
    ["notion", "#191919"],
    ["slack", "#4a154b"],
    ["gmail", "#ea4335"],
] as const;

/** Draw the connectors the composer has on: one per vendor, or your space alone. */
function Connectors(properties: { isOpen: boolean }) {
    return (
        <span {...style.attrs(styles.connectors)}>
            <span {...style.attrs(styles.logos)}>
                {properties.isOpen ? (
                    <span {...style.attrs(styles.logo)}>
                        <img alt="" src="/brand/mark.svg" {...style.attrs(styles.server)} />
                    </span>
                ) : (
                    vendors.map(([icon, tint]) => (
                        <span {...style.attrs(styles.logo)}>
                            <Favicon icon={icon} tint={tint} size={11} />
                        </span>
                    ))
                )}
            </span>
            {properties.isOpen ? "Destack" : `${vendors.length} connectors`}
        </span>
    );
}

/** Draw one assistant's part of the chrome, faded in while that assistant is shown. */
function Layer(properties: { isShown: boolean; children: JSX.Element }) {
    return (
        <span
            aria-hidden={properties.isShown ? undefined : "true"}
            {...style.attrs(styles.layer, properties.isShown && styles.layerShown)}
        >
            {properties.children}
        </span>
    );
}

/** Draw one item of the conversation in its kind's shape. */
function ItemView(properties: { item: Item; assistant: Assistant }) {
    const item = properties.item;

    // your message, in a bubble on the right
    if (item.kind === "ask") {
        return <span {...style.attrs(styles.ask)}>{item.text}</span>;
    }
    // a tool the assistant ran, as a one-line summary that opens onto its request and response
    else if (item.kind === "call") {
        return <ToolCall item={item} />;
    }
    // a request for your approval, or its outcome
    else if (item.kind === "approval") {
        return (
            <span {...style.attrs(styles.approval, item.isApproved && styles.approved)}>
                <span>{item.isApproved ? item.text : `${properties.assistant} ${item.text}`}</span>
                {item.isApproved ? undefined : (
                    <span {...style.attrs(styles.buttons)}>
                        <span {...style.attrs(styles.button)}>Deny</span>
                        <span {...style.attrs(styles.button, styles.allow)}>Approve</span>
                    </span>
                )}
            </span>
        );
    }

    // the assistant's reply, in its type
    return <span {...style.attrs(styles.reply)}>{item.text}</span>;
}

/** Draw a tool the assistant ran: its summary line, and its request and response once opened. */
function ToolCall(properties: { item: Extract<Item, { kind: "call" }> }) {
    const [isOpen, setIsOpen] = createSignal(false);
    const item = () => properties.item;

    return (
        <span {...style.attrs(styles.tool)}>
            <button
                type="button"
                aria-expanded={isOpen() ? "true" : "false"}
                title={serverNames[item().server]}
                onClick={() => setIsOpen(!isOpen())}
                {...style.attrs(styles.toolHead)}
            >
                {item().server === "destack" ? (
                    <img alt="" src="/brand/mark.svg" {...style.attrs(styles.server)} />
                ) : (
                    <Favicon icon={item().server} tint={chat.ink} size={13} />
                )}
                <span {...style.attrs(styles.toolTitle)}>{item().title}</span>
                {item().isFailed ? <span {...style.attrs(styles.failed)}>failed</span> : undefined}
                <span {...style.attrs(styles.chevron, isOpen() && styles.chevronOpen)}>
                    <Glyph name="caret" size={10} />
                </span>
            </button>
            {isOpen() ? (
                <span {...style.attrs(styles.toolBody)}>
                    <span {...style.attrs(styles.fieldLabel)}>Request</span>
                    <code {...style.attrs(styles.code)}>{item().request}</code>
                    <span {...style.attrs(styles.fieldLabel)}>Response</span>
                    <code
                        {...style.attrs(
                            styles.code,
                            item().isFailed ? styles.failed : styles.passed,
                        )}
                    >
                        {item().response}
                    </code>
                </span>
            ) : undefined}
        </span>
    );
}

/** The time a change of assistant takes to ease in. */
const EASE = "560ms";

/** The colour and shape properties that ease between assistants. */
const EASED = "background-color, color, border-color, border-radius, box-shadow";

/** The conversation settling into a new assistant's type, after a change from Claude or Gemini. */
const settleEven = style.keyframes({
    from: { filter: "blur(2px)", opacity: 0.4 },
    to: { filter: "blur(0)", opacity: 1 },
});

/** The same settle under a second name, so each change of assistant runs it afresh. */
const settleOdd = style.keyframes({
    from: { filter: "blur(2px)", opacity: 0.35 },
    to: { filter: "blur(0)", opacity: 1 },
});

/** Each assistant's conversation, as it ships. */
const themes = style.create({
    Claude: {},
    ChatGPT: {
        [chat.canvas]: chatgpt.canvas,
        [chat.ink]: chatgpt.ink,
        [chat.muted]: chatgpt.muted,
        [chat.border]: chatgpt.border,
        [chat.field]: chatgpt.field,
        [chat.bubble]: chatgpt.bubble,
        [chat.bubbleRadius]: "18px",
        [chat.bubbleCorner]: "18px",
        [chat.replyFont]: SANS,
        [chat.replySize]: "0.9375rem",
        [chat.replyLine]: "1.7",
    },
    Gemini: {
        [chat.canvas]: gemini.canvas,
        [chat.ink]: gemini.ink,
        [chat.muted]: gemini.muted,
        [chat.border]: gemini.border,
        [chat.field]: gemini.field,
        [chat.bubble]: gemini.bubble,
        [chat.bubbleRadius]: "22px",
        [chat.bubbleCorner]: "4px",
        [chat.replyFont]: SANS,
        [chat.replySize]: "0.9375rem",
        [chat.replyLine]: "1.65",
    },
});

/** The assistant styles. */
const styles = style.create({
    window: {
        backgroundColor: chat.canvas,
        boxShadow: "none",
        color: chat.ink,
        display: "grid",
        fontFamily: SANS,
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        height: "100%",
        transitionDuration: EASE,
        transitionProperty: EASED,
    },

    // layers
    stack: {
        display: "grid",
    },
    layer: {
        display: "grid",
        gridArea: "1 / 1",
        opacity: 0,
        pointerEvents: "none",
        transitionDuration: EASE,
        transitionProperty: "opacity",
    },
    layerShown: {
        opacity: 1,
        pointerEvents: "auto",
    },

    // title bars
    bar: {
        alignItems: "center",
        color: chat.muted,
        display: "flex",
        gap: "0.875rem",
        height: "2.625rem",
        paddingInline: "0.875rem",
    },
    conversation: {
        alignItems: "center",
        color: chat.ink,
        display: "inline-flex",
        fontSize: "0.8125rem",
        gap: "0.25rem",
    },
    share: {
        borderColor: chat.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: chat.ink,
        fontSize: "0.75rem",
        paddingBlock: "0.25rem",
        paddingInline: "0.625rem",
    },
    chatgptShare: {
        alignItems: "center",
        borderRadius: "999px",
        display: "inline-flex",
        gap: "0.375rem",
    },
    barEnd: {
        alignItems: "center",
        color: chat.muted,
        display: "inline-flex",
        gap: "0.875rem",
        marginInlineStart: "auto",
    },
    avatar: {
        alignItems: "center",
        backgroundColor: gemini.avatar,
        borderRadius: "50%",
        color: assistant.white,
        display: "inline-flex",
        fontSize: "0.6875rem",
        fontWeight: 600,
        height: "1.5rem",
        justifyContent: "center",
        width: "1.5rem",
    },

    // conversation
    messages: {
        animationDuration: EASE,
        animationTimingFunction: "ease-out",
        display: "flex",
        flexDirection: "column",
        gap: "1rem",
        justifyContent: "flex-end",
        maskImage: "linear-gradient(to bottom, transparent, black 1.5rem)",
        minHeight: 0,
        overflow: "hidden",
        paddingBlock: "0.5rem",
        paddingInline: "1.5rem",
    },
    settleEven: {
        animationName: settleEven,
    },
    settleOdd: {
        animationName: settleOdd,
    },
    ask: {
        alignSelf: "flex-end",
        backgroundColor: chat.bubble,
        borderBottomLeftRadius: chat.bubbleRadius,
        borderBottomRightRadius: chat.bubbleRadius,
        borderTopLeftRadius: chat.bubbleRadius,
        borderTopRightRadius: chat.bubbleCorner,
        flexShrink: 0,
        fontSize: "0.875rem",
        lineHeight: 1.5,
        marginTop: "0.75rem",
        maxWidth: "80%",
        paddingBlock: "0.625rem",
        paddingInline: "1rem",
        transitionDuration: EASE,
        transitionProperty: EASED,
    },
    reply: {
        flexShrink: 0,
        fontFamily: chat.replyFont,
        fontSize: chat.replySize,
        lineHeight: chat.replyLine,
    },
    footer: {
        color: chat.muted,
        flexShrink: 0,
        justifyItems: "start",
        paddingTop: "0.125rem",
    },
    actions: {
        alignItems: "center",
        display: "inline-flex",
        gap: "0.75rem",
        height: "18px",
    },

    // tool calls
    tool: {
        display: "grid",
        flexShrink: 0,
        gap: "0.375rem",
    },
    toolHead: {
        alignItems: "center",
        opacity: { default: null, ":hover": { default: null, [media.hover]: 0.75 } },
        backgroundColor: "transparent",
        borderWidth: 0,
        color: chat.muted,
        cursor: "pointer",
        display: "flex",
        fontFamily: "inherit",
        fontSize: "0.75rem",
        gap: "0.5rem",
        paddingBlock: "0.125rem",
        paddingInline: 0,
        textAlign: "start",
        width: "100%",
    },
    server: {
        borderRadius: "3px",
        flexShrink: 0,
        height: "13px",
        width: "13px",
    },
    toolTitle: {
        color: chat.ink,
        fontWeight: 500,
    },
    chevron: {
        display: "inline-flex",
        transitionDuration: "160ms",
        transitionProperty: "transform",
    },
    chevronOpen: {
        transform: "rotate(180deg)",
    },
    toolBody: {
        backgroundColor: chat.field,
        borderColor: chat.border,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "0.25rem",
        paddingBlock: "0.5rem",
        paddingInline: "0.75rem",
        transitionDuration: EASE,
        transitionProperty: EASED,
    },
    fieldLabel: {
        color: chat.muted,
        fontSize: "0.625rem",
        fontWeight: 600,
        letterSpacing: "0.04em",
        textTransform: "uppercase",
    },
    code: {
        fontFamily: font.code,
        fontSize: "0.6875rem",
        lineHeight: 1.5,
        overflowWrap: "anywhere",
    },
    passed: {
        color: assistant.passed,
    },
    failed: {
        color: assistant.failed,
    },

    // approvals
    approval: {
        borderColor: chat.border,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        flexShrink: 0,
        fontSize: "0.8125rem",
        gap: "0.625rem",
        paddingBlock: "0.75rem",
        paddingInline: "0.875rem",
        transitionDuration: EASE,
        transitionProperty: EASED,
    },
    approved: {
        color: assistant.passed,
    },
    buttons: {
        display: "flex",
        gap: "0.375rem",
        justifyContent: "end",
    },
    button: {
        borderColor: chat.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "0.75rem",
        fontWeight: 500,
        paddingBlock: "0.3125rem",
        paddingInline: "0.875rem",
        transitionDuration: EASE,
        transitionProperty: EASED,
    },
    allow: {
        backgroundColor: chat.ink,
        borderColor: chat.ink,
        color: chat.canvas,
    },

    // composers
    composers: {
        alignItems: "end",
        paddingBlockEnd: "0.875rem",
        paddingBlockStart: "0.75rem",
        paddingInline: "0.875rem",
    },
    composer: {
        alignItems: "center",
        color: chat.muted,
        gap: "0.625rem",
        paddingBlock: "0.75rem",
        paddingInline: "0.875rem",
    },
    claudeComposer: {
        backgroundColor: claude.composer,
        borderColor: claude.edge,
        borderRadius: "18px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: claude.lift,
        display: "grid",
    },
    chatgptComposer: {
        backgroundColor: chatgpt.composer,
        borderRadius: "28px",
        boxShadow: chatgpt.lift,
        color: chat.ink,
        display: "grid",
        paddingBlock: "0.875rem",
        paddingInline: "1rem",
    },
    geminiComposer: {
        backgroundColor: gemini.composer,
        borderRadius: "28px",
        display: "grid",
        paddingBlock: "1rem",
        paddingInline: "1.25rem",
    },
    connectors: {
        alignItems: "center",
        borderColor: chat.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: chat.muted,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.375rem",
        paddingBlock: "0.125rem",
        paddingInlineEnd: "0.5rem",
        paddingInlineStart: "0.25rem",
        whiteSpace: "nowrap",
    },
    logos: {
        display: "inline-flex",
    },
    logo: {
        alignItems: "center",
        backgroundColor: chat.canvas,
        borderColor: chat.border,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        height: "18px",
        justifyContent: "center",
        marginInlineStart: { default: "-5px", ":first-child": 0 },
        width: "18px",
    },
    idle: {
        opacity: 0.5,
    },
    placeholder: {
        color: chat.muted,
        fontSize: "0.875rem",
    },
    grow: {
        flexGrow: 1,
    },
    composerRow: {
        alignItems: "center",
        display: "flex",
        gap: "0.625rem",
    },
    hint: {
        alignItems: "center",
        color: chat.muted,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.25rem",
        whiteSpace: "nowrap",
    },
    end: {
        marginInlineStart: "auto",
    },
    claudePlus: {
        alignItems: "center",
        borderColor: claude.edge,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        height: "1.75rem",
        justifyContent: "center",
        width: "1.75rem",
    },
    claudeSend: {
        alignItems: "center",
        backgroundColor: assistant.clay,
        borderRadius: "8px",
        color: assistant.white,
        display: "inline-flex",
        height: "1.75rem",
        justifyContent: "center",
        width: "1.75rem",
    },
    chatgptSend: {
        alignItems: "center",
        backgroundColor: chatgpt.send,
        borderRadius: "50%",
        color: chatgpt.sendInk,
        display: "inline-flex",
        height: "2rem",
        justifyContent: "center",
        width: "2rem",
    },
    geminiTools: {
        alignItems: "center",
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.375rem",
    },
});
