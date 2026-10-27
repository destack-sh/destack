import { font, motion } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createSignal, For, type JSX, Show } from "@destack/view";

import { browserStyles } from "../figure/browser";
import { Favicon, Glyph } from "../figure/glyph";
import type { Option } from "../figure/picker";
import { type Tool, tools } from "../tool/tool";
import { assistant, chat, chatgpt, claude, gemini, grok } from "./chat.stylex";
import {
    BEAT,
    CHARACTERS_PER_BEAT,
    type Grant,
    type Item,
    type Moment,
    WORDS_PER_BEAT,
} from "./scene";

/** The sans the assistants' chrome and your messages set in. */
const SANS = 'ui-sans-serif, -apple-system, "Segoe UI", system-ui, sans-serif';

/** The connector logos the composer shows before its count, as many as fit a narrow chat. */
const SHOWN_CONNECTORS = 3;

/** The assistants the chat takes the look of, in turn. */
export const assistants = ["Claude", "ChatGPT", "Gemini", "Grok"] as const;

/** Each assistant as the picker offers it, by its mark and colour. */
export const assistantOptions: readonly Option[] = [
    { name: "Claude", mark: { icon: "claude", tint: "#c96442" } },
    { name: "ChatGPT", mark: { icon: "openai", tint: "#000000" } },
    { name: "Gemini", mark: { icon: "gemini", tint: "#4285f4" } },
    { name: "Grok", mark: { icon: "grok", tint: "#000000" } },
];

/** One assistant the chat can look like. */
export type Assistant = (typeof assistants)[number];

/** Return the assistant a chat looks like: the reader's choice, or the next one each run. */
export function createAssistantTurn(
    beat: () => number,
    length: () => number,
    isReduced: () => boolean,
) {
    // take the assistant the reader chose, or else the next one's look each time the task runs again
    const [chosen, setChosen] = createSignal<Assistant | undefined>(undefined);
    const turned = () =>
        present(assistants[Math.floor(beat() / length()) % assistants.length], "turned assistant");
    const look = () => chosen() ?? turned();

    // show how far the run has come until the reader chooses
    const progress = () =>
        chosen() === undefined && !isReduced() ? (beat() % length()) / length() : undefined;
    const choose = (name: string) =>
        setChosen(
            present(
                assistants.find((candidate) => candidate === name),
                "chosen assistant",
            ),
        );

    return { look, progress, choose };
}

/** Draw an assistant's desktop app a turn at a time: through connectors, or with your space. */
export function AgentChat(properties: {
    isOpen: boolean;
    moment: Moment;
    assistant: Assistant;
    connectors: readonly Connector[];
}) {
    // tell each assistant's chrome apart, so the shown one fades in over the rest
    const isShown = (candidate: Assistant) => candidate === properties.assistant;

    // send once a message is typed, stop while the assistant works, and rest otherwise
    const send = (): Send =>
        properties.moment.isWorking
            ? "stop"
            : properties.moment.typing === undefined
              ? "idle"
              : "ready";

    // show the assistant at work under the conversation, and its actions once it replied
    const isReplied = () => properties.moment.items.some((item) => item.kind === "reply");

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
                <Layer isShown={isShown("Grok")}>
                    <span {...style.attrs(styles.bar)}>
                        <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                        <Glyph name="sidebar" size={15} />
                        <span {...style.attrs(styles.conversation)}>Launch day</span>
                        <span {...style.attrs(styles.barEnd)}>
                            <Glyph name="share" size={14} />
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
                <For each={properties.moment.items}>
                    {(item) => (
                        <ItemView
                            item={item}
                            assistant={properties.assistant}
                            isRunning={item === properties.moment.running}
                            isDeciding={item === properties.moment.deciding}
                            isPressed={item === properties.moment.pressed}
                            isBuilding={item === properties.moment.building}
                        />
                    )}
                </For>

                {/* each assistant at work, then its mark or actions under its reply */}
                <span {...style.attrs(styles.stack, styles.footer)}>
                    <Layer isShown={isShown("Claude")}>
                        {properties.moment.isWorking ? (
                            <span {...style.attrs(styles.spinning)}>
                                <Favicon icon="claude" tint={assistant.clay} size={18} />
                            </span>
                        ) : isReplied() ? (
                            <Favicon icon="claude" tint={assistant.clay} size={18} />
                        ) : undefined}
                    </Layer>
                    <Layer isShown={isShown("ChatGPT")}>
                        {properties.moment.isWorking ? (
                            <span {...style.attrs(styles.pulse)} />
                        ) : isReplied() ? (
                            <span {...style.attrs(styles.actions)}>
                                <Glyph name="copy" size={14} />
                                <Glyph name="like" size={14} />
                                <Glyph name="dislike" size={14} />
                                <Glyph name="reload" size={14} />
                            </span>
                        ) : undefined}
                    </Layer>
                    <Layer isShown={isShown("Gemini")}>
                        {properties.moment.isWorking ? (
                            <span {...style.attrs(styles.spinning)}>
                                <Favicon icon="gemini" tint={gemini.spark} size={18} />
                            </span>
                        ) : isReplied() ? (
                            <span {...style.attrs(styles.actions)}>
                                <Glyph name="like" size={14} />
                                <Glyph name="dislike" size={14} />
                                <Glyph name="share" size={14} />
                                <Glyph name="more" size={14} />
                            </span>
                        ) : undefined}
                    </Layer>
                    <Layer isShown={isShown("Grok")}>
                        {properties.moment.isWorking ? (
                            <span {...style.attrs(styles.spinning)}>
                                <Favicon icon="grok" tint={grok.ink} size={18} />
                            </span>
                        ) : isReplied() ? (
                            <span {...style.attrs(styles.actions)}>
                                <Glyph name="copy" size={14} />
                                <Glyph name="reload" size={14} />
                                <Glyph name="like" size={14} />
                                <Glyph name="dislike" size={14} />
                            </span>
                        ) : undefined}
                    </Layer>
                </span>
            </div>

            {/* composer, one per assistant */}
            <div {...style.attrs(styles.stack, styles.composers)}>
                <Layer isShown={isShown("Claude")}>
                    <span {...style.attrs(styles.composer, styles.claudeComposer)}>
                        <Typed text={properties.moment.typing} placeholder="Reply to Claude…" />
                        <span {...style.attrs(styles.composerRow)}>
                            <span {...style.attrs(styles.claudePlus)}>
                                <Glyph name="plus" size={14} />
                            </span>
                            <span {...style.attrs(styles.claudePlus)}>
                                <Glyph name="sliders" size={14} />
                            </span>
                            <Connectors
                                isOpen={properties.isOpen}
                                connectors={properties.connectors}
                            />
                            <span {...style.attrs(styles.hint, styles.end)}>
                                Opus 5.5
                                <Glyph name="caret" size={10} />
                            </span>
                            <ClaudeSend send={send()} />
                        </span>
                    </span>
                </Layer>
                <Layer isShown={isShown("ChatGPT")}>
                    <span {...style.attrs(styles.composer, styles.chatgptComposer)}>
                        <Typed text={properties.moment.typing} placeholder="Ask anything" />
                        <span {...style.attrs(styles.composerRow)}>
                            <Glyph name="plus" size={18} />
                            <Connectors
                                isOpen={properties.isOpen}
                                connectors={properties.connectors}
                            />
                            <span {...style.attrs(styles.end)}>
                                <Glyph name="microphone" size={16} />
                            </span>
                            <span {...style.attrs(styles.chatgptSend)}>
                                {send() === "idle" ? (
                                    <Glyph name="waveform" size={15} weight={2.25} />
                                ) : (
                                    <SendMark send={send()} />
                                )}
                            </span>
                        </span>
                    </span>
                </Layer>
                <Layer isShown={isShown("Gemini")}>
                    <span {...style.attrs(styles.composer, styles.geminiComposer)}>
                        <Typed text={properties.moment.typing} placeholder="Ask Gemini" />
                        <span {...style.attrs(styles.composerRow)}>
                            <Glyph name="plus" size={16} />
                            <span {...style.attrs(styles.geminiTools)}>
                                <Glyph name="sliders" size={14} />
                                Tools
                            </span>
                            <Connectors
                                isOpen={properties.isOpen}
                                connectors={properties.connectors}
                            />
                            <span {...style.attrs(styles.end)}>
                                {send() === "idle" ? (
                                    <Glyph name="microphone" size={18} />
                                ) : (
                                    <span {...style.attrs(styles.geminiSend)}>
                                        <SendMark send={send()} />
                                    </span>
                                )}
                            </span>
                        </span>
                    </span>
                </Layer>
                <Layer isShown={isShown("Grok")}>
                    <span {...style.attrs(styles.composer, styles.grokComposer)}>
                        <Typed text={properties.moment.typing} placeholder="Ask Grok anything" />
                        <span {...style.attrs(styles.composerRow)}>
                            <Glyph name="plus" size={18} />
                            <Connectors
                                isOpen={properties.isOpen}
                                connectors={properties.connectors}
                            />
                            <span {...style.attrs(styles.hint, styles.end)}>
                                Fast
                                <Glyph name="caret" size={10} />
                            </span>
                            <span {...style.attrs(styles.grokSend)}>
                                {send() === "idle" ? (
                                    <Glyph name="waveform" size={15} weight={2.25} />
                                ) : (
                                    <SendMark send={send()} />
                                )}
                            </span>
                        </span>
                    </span>
                </Layer>
            </div>
        </div>
    );
}

/** A connector the assistant reaches an app through: its mark and colour. */
export type Connector = readonly [icon: string, tint: string];

/** Draw the connectors the composer has on: one per vendor, or your space alone. */
function Connectors(properties: { isOpen: boolean; connectors: readonly Connector[] }) {
    return (
        <span {...style.attrs(styles.connectors)}>
            <span {...style.attrs(styles.logos)}>
                {properties.isOpen ? (
                    <span {...style.attrs(styles.logo)}>
                        <img
                            alt=""
                            src="/brand/favicon/favicon.svg"
                            {...style.attrs(styles.server)}
                        />
                    </span>
                ) : (
                    properties.connectors.slice(0, SHOWN_CONNECTORS).map(([icon, tint]) => (
                        <span {...style.attrs(styles.logo)}>
                            <Favicon icon={icon} tint={tint} size={13} />
                        </span>
                    ))
                )}
            </span>
            {properties.isOpen ? "Destack" : `${properties.connectors.length} connectors`}
        </span>
    );
}

/** The state of the composer's send button: resting empty, ready to send, or stopping a reply. */
type Send = "idle" | "ready" | "stop";

/** Draw the send button's mark: an arrow, or the square that stops the reply. */
function SendMark(properties: { send: Send }) {
    return (
        <>
            {properties.send === "stop" ? (
                <span {...style.attrs(styles.stop)} />
            ) : (
                <Glyph name="up" size={14} weight={2.5} />
            )}
        </>
    );
}

/** Draw Claude's send button, faded while the composer is empty. */
function ClaudeSend(properties: { send: Send }) {
    return (
        <span {...style.attrs(styles.claudeSend, properties.send === "idle" && styles.idle)}>
            <SendMark send={properties.send} />
        </span>
    );
}

/** Draw your message typing into the composer a character at a time, or else the placeholder. */
function Typed(properties: { text: string | undefined; placeholder: string }) {
    return (
        <Show
            when={properties.text}
            fallback={<span {...style.attrs(styles.placeholder)}>{properties.placeholder}</span>}
        >
            {(text) => (
                <span {...style.attrs(styles.typed)}>
                    {Array.from(text(), (character, index) => (
                        <span
                            style={{
                                "animation-delay": `${(index * BEAT) / CHARACTERS_PER_BEAT}ms`,
                            }}
                            {...style.attrs(styles.character)}
                        >
                            {character}
                        </span>
                    ))}
                </span>
            )}
        </Show>
    );
}

/** Draw the assistant's reply streaming in a word at a time. */
function Streamed(properties: { text: string }) {
    return (
        <>
            {properties.text.split(" ").map((word, index) => (
                <>
                    {index > 0 && " "}
                    <span
                        style={{ "animation-delay": `${(index * BEAT) / WORDS_PER_BEAT}ms` }}
                        {...style.attrs(styles.word)}
                    >
                        {word}
                    </span>
                </>
            ))}
        </>
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
function ItemView(properties: {
    item: Item;
    assistant: Assistant;
    isRunning: boolean;
    isDeciding: boolean;
    isPressed: boolean;
    isBuilding: boolean;
}) {
    const item = properties.item;

    // your message, in a bubble on the right
    if (item.kind === "ask") {
        return <span {...style.attrs(styles.ask)}>{item.text}</span>;
    }
    // a tool the assistant ran, as a one-line summary that opens onto its request and response
    else if (item.kind === "call") {
        return <ToolCall item={item} isRunning={properties.isRunning} />;
    }
    // an app the assistant made, its note landing once it is built
    else if (item.kind === "app") {
        return item.isLive ? (
            <AppCard item={item} isBuilding={properties.isBuilding} />
        ) : (
            <ArtifactCard
                item={item}
                assistant={properties.assistant}
                isBuilding={properties.isBuilding}
            />
        );
    }
    // a request for your approval while you read it, then its outcome
    else if (item.kind === "approval") {
        return (
            <span {...style.attrs(styles.approval, !properties.isDeciding && styles.approved)}>
                <span>
                    {properties.isDeciding ? `${properties.assistant} ${item.text}` : item.outcome}
                </span>
                {properties.isDeciding ? (
                    <span {...style.attrs(styles.buttons)}>
                        <span {...style.attrs(styles.button)}>Deny</span>
                        <span
                            {...style.attrs(
                                styles.button,
                                styles.allow,
                                properties.isPressed && styles.buttonPressed,
                            )}
                        >
                            {allowOf(item.grant, properties.assistant)}
                        </span>
                    </span>
                ) : undefined}
            </span>
        );
    }

    // the assistant's reply, in its type, streamed in
    return (
        <span {...style.attrs(styles.reply)}>
            <Streamed text={item.text} />
        </span>
    );
}

/** Draw a tool the assistant ran: its summary line, and its request and response once opened. */
function ToolCall(properties: { item: Extract<Item, { kind: "call" }>; isRunning: boolean }) {
    const [isOpen, setIsOpen] = createSignal(properties.item.isExpanded);
    const item = () => properties.item;

    return (
        <span {...style.attrs(styles.tool)}>
            <button
                type="button"
                aria-expanded={isOpen() ? "true" : "false"}
                title={item().server === "destack" ? "Destack" : vendorOf(item().server).name}
                onClick={() => setIsOpen(!isOpen())}
                {...style.attrs(styles.toolHead)}
            >
                {item().server === "destack" ? (
                    <img alt="" src="/brand/favicon/favicon.svg" {...style.attrs(styles.server)} />
                ) : (
                    <Favicon icon={item().server} tint={vendorOf(item().server).tint} size={13} />
                )}
                {properties.isRunning ? (
                    <span {...style.attrs(styles.toolTitle, styles.shimmer)}>{item().running}</span>
                ) : (
                    <>
                        <span {...style.attrs(styles.toolTitle)}>{item().title}</span>
                        {item().isFailed ? (
                            <span {...style.attrs(styles.failed)}>failed</span>
                        ) : undefined}
                        <span {...style.attrs(styles.chevron, isOpen() && styles.chevronOpen)}>
                            <Glyph name="caret" size={10} />
                        </span>
                    </>
                )}
            </button>
            {isOpen() && !properties.isRunning ? (
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

/** Return the tool a vendor's connector serves, by the tool's mark. */
function vendorOf(server: string): Tool {
    return present(
        tools.find((tool) => tool.icon === server),
        `tool with the mark ${server}`,
    );
}

/**
 * Draw an app the assistant made inside the chat: a board, live in your space or an artifact.
 *
 * Its note lands once it is built: who it is shared with, and as whom it acts.
 */
function AppCard(properties: { item: Extract<Item, { kind: "app" }>; isBuilding: boolean }) {
    return (
        <span data-component="AppCard" {...style.attrs(styles.app)}>
            <span {...style.attrs(styles.appHead)}>
                <b>{properties.item.title}</b>
                <span {...style.attrs(styles.appBadge, properties.item.isLive && styles.appLive)}>
                    {properties.item.isLive ? <span {...style.attrs(styles.appDot)} /> : undefined}
                    {properties.item.badge}
                </span>
            </span>
            {properties.item.rows.map(([name, asked, state]) => (
                <span {...style.attrs(styles.appRow)}>
                    <span {...style.attrs(styles.appTitle)}>{name}</span>
                    <span {...style.attrs(styles.appMeta, styles.appAsked)}>{asked}</span>
                    <span {...style.attrs(styles.appState)}>{state}</span>
                </span>
            ))}
            {properties.isBuilding ? undefined : (
                <span {...style.attrs(styles.appFoot, properties.item.isLive && styles.appShared)}>
                    {properties.item.note}
                </span>
            )}
        </span>
    );
}

/** Draw an app the assistant made as its artifact: a sandboxed preview it keeps in the chat. */
function ArtifactCard(properties: {
    item: Extract<Item, { kind: "app" }>;
    assistant: Assistant;
    isBuilding: boolean;
}) {
    return (
        <span data-component="ArtifactCard" {...style.attrs(styles.app)}>
            {/* the artifact's bar: its mark, title and kind, its tabs, and publishing */}
            <span {...style.attrs(styles.artifactBar)}>
                <span aria-hidden="true" {...style.attrs(styles.artifactMark)}>
                    {"</>"}
                </span>
                <span {...style.attrs(styles.artifactName)}>
                    <b {...style.attrs(styles.appTitle)}>{properties.item.title}</b>
                    <span {...style.attrs(styles.appMeta)}>
                        {kindOf(properties.assistant)} · {properties.item.badge}
                    </span>
                </span>
                <span {...style.attrs(styles.artifactTabs)}>
                    <span {...style.attrs(styles.artifactTab, styles.artifactTabOn)}>Preview</span>
                    <span {...style.attrs(styles.artifactTab)}>Code</span>
                </span>
                <span {...style.attrs(styles.artifactPublish)}>Publish</span>
            </span>

            {/* the preview */}
            {properties.item.rows.map(([name, asked, state]) => (
                <span {...style.attrs(styles.appRow)}>
                    <span {...style.attrs(styles.appTitle)}>{name}</span>
                    <span {...style.attrs(styles.appMeta, styles.appAsked)}>{asked}</span>
                    <span {...style.attrs(styles.appState)}>{state}</span>
                </span>
            ))}

            {/* where it runs and as whom, once it is built */}
            {properties.isBuilding ? undefined : (
                <span {...style.attrs(styles.appFoot)}>{properties.item.note}</span>
            )}
        </span>
    );
}

/** Return what an assistant calls an app it makes in the chat. */
function kindOf(shown: Assistant): string {
    return shown === "ChatGPT" || shown === "Gemini" ? "Canvas" : "Artifact";
}

/** Return an approval's allow label, by how long it holds and whose chrome shows it. */
function allowOf(grant: Grant, shown: Assistant): string {
    // hold for the rest of the chat
    if (grant === "chat") {
        return "Allow for this chat";
    }

    // hold for this call alone
    return shown === "Claude" ? "Allow once" : "Approve";
}

/** The colour and shape properties that ease between assistants. */
const EASED = "background-color, color, border-color, border-radius, box-shadow";

/** The conversation settling into a new assistant's type, after a change from Claude or Gemini. */
const settleEven = style.keyframes({
    from: { opacity: 0.4 },
    to: { opacity: 1 },
});

/** The same settle under a second name, so each change of assistant runs it afresh. */
const settleOdd = style.keyframes({
    from: { opacity: 0.35 },
    to: { opacity: 1 },
});

/** A typed character or a streamed word appearing, in place. */
const appear = style.keyframes({
    from: { opacity: 0 },
    to: { opacity: 1 },
});

/** The light sweeping across a running tool's title. */
const sweep = style.keyframes({
    from: { backgroundPosition: "100% 0" },
    to: { backgroundPosition: "-100% 0" },
});

/** A mark turning while its assistant works. */
const spin = style.keyframes({
    from: { transform: "rotate(0deg)" },
    to: { transform: "rotate(360deg)" },
});

/** A dot breathing while its assistant works. */
const breathe = style.keyframes({
    "0%, 100%": { transform: "scale(0.7)" },
    "50%": { transform: "scale(1)" },
});

/** Each assistant's conversation, as it ships. */
export const themes = style.create({
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
    Grok: {
        [chat.canvas]: grok.canvas,
        [chat.ink]: grok.ink,
        [chat.muted]: grok.muted,
        [chat.border]: grok.border,
        [chat.field]: grok.field,
        [chat.bubble]: grok.bubble,
        [chat.bubbleRadius]: "24px",
        [chat.bubbleCorner]: "8px",
        [chat.replyFont]: SANS,
        [chat.replySize]: "0.9375rem",
        [chat.replyLine]: "1.65",
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
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
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
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
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
        animationDuration: motion.durationMedium,
        animationTimingFunction: motion.easingStandard,
        display: "flex",
        flexDirection: "column",
        gap: "1rem",
        contain: "layout paint",
        justifyContent: "flex-end",
        minHeight: 0,
        overflow: "hidden",
        paddingBlock: "0.5rem",
        paddingInline: "1.5rem",
        position: "relative",
        // fade the oldest messages out under a gradient, cheaper than masking a changing list
        "::before": {
            backgroundImage: `linear-gradient(to bottom, ${chat.canvas}, transparent)`,
            content: '""',
            height: "1.5rem",
            insetInline: 0,
            pointerEvents: "none",
            position: "absolute",
            top: 0,
            zIndex: 1,
        },
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
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
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

    // arrival and work
    typed: {
        color: chat.ink,
        fontSize: "0.875rem",
        lineHeight: 1.5,
    },
    character: {
        animationDuration: motion.durationShort,
        animationFillMode: "backwards",
        animationName: appear,
    },
    word: {
        animationDuration: motion.durationMedium,
        animationFillMode: "backwards",
        animationName: appear,
        animationTimingFunction: motion.easingEmphasised,
    },
    shimmer: {
        animationDuration: { default: "1.6s", [media.motionReduce]: "0ms" },
        animationIterationCount: "infinite",
        animationName: sweep,
        animationTimingFunction: "linear",
        backgroundClip: "text",
        backgroundImage: `linear-gradient(90deg, ${chat.muted} 35%, ${chat.ink} 50%, ${chat.muted} 65%)`,
        backgroundSize: "200% 100%",
        color: "transparent",
    },
    spinning: {
        animationDuration: { default: "2.4s", [media.motionReduce]: "0ms" },
        animationIterationCount: "infinite",
        animationName: spin,
        animationTimingFunction: "linear",
        display: "inline-flex",
    },
    pulse: {
        animationDuration: { default: "1.2s", [media.motionReduce]: "0ms" },
        animationIterationCount: "infinite",
        animationName: breathe,
        backgroundColor: chat.ink,
        borderRadius: "50%",
        height: "12px",
        width: "12px",
    },
    stop: {
        backgroundColor: "currentColor",
        borderRadius: "2px",
        height: "10px",
        width: "10px",
    },
    buttonPressed: {
        opacity: 0.7,
        transform: "scale(0.96)",
    },
    geminiSend: {
        alignItems: "center",
        backgroundColor: gemini.send,
        borderRadius: "50%",
        color: gemini.sendInk,
        display: "inline-flex",
        height: "2rem",
        justifyContent: "center",
        width: "2rem",
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
        borderRadius: "4px",
        flexShrink: 0,
        height: "16px",
        width: "16px",
    },
    toolTitle: {
        color: chat.ink,
        fontWeight: 500,
    },
    chevron: {
        display: "inline-flex",
        transitionDuration: motion.durationShort,
        transitionProperty: "transform",
        transitionTimingFunction: motion.easingStandard,
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
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
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
        whiteSpace: "pre-wrap",
    },
    passed: {
        color: assistant.passed,
    },
    failed: {
        color: assistant.failed,
    },

    // files

    // apps
    app: {
        backgroundColor: chat.canvas,
        borderColor: chat.border,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        flexShrink: 0,
        fontSize: "0.8125rem",
        overflow: "hidden",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
        transitionProperty: EASED,
    },
    appHead: {
        alignItems: "center",
        borderBottomColor: chat.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        justifyContent: "space-between",
        paddingBlock: "0.5rem",
        paddingInline: "0.75rem",
    },
    appBadge: {
        alignItems: "center",
        borderColor: chat.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        fontFamily: font.code,
        fontSize: "0.625rem",
        gap: "0.3125rem",
        opacity: 0.7,
        paddingBlock: "0.0625rem",
        paddingInline: "0.5rem",
    },
    appLive: {
        borderColor: `color-mix(in srgb, ${assistant.passed} 45%, transparent)`,
        color: assistant.passed,
        opacity: 1,
    },
    appDot: {
        backgroundColor: assistant.passed,
        borderRadius: "50%",
        height: "6px",
        width: "6px",
    },
    artifactBar: {
        alignItems: "center",
        borderBottomColor: chat.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "0.5rem",
        paddingBlock: "0.5rem",
        paddingInline: "0.625rem",
    },
    artifactMark: {
        alignItems: "center",
        backgroundColor: `color-mix(in srgb, ${chat.ink} 8%, transparent)`,
        borderRadius: "6px",
        display: "inline-flex",
        flexShrink: 0,
        fontFamily: font.code,
        fontSize: "0.625rem",
        fontWeight: 600,
        height: "1.75rem",
        justifyContent: "center",
        width: "1.75rem",
    },
    artifactName: {
        display: "grid",
        flexGrow: 1,
        minWidth: 0,
    },
    artifactTabs: {
        backgroundColor: `color-mix(in srgb, ${chat.ink} 6%, transparent)`,
        borderRadius: "6px",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "0.6875rem",
        padding: "2px",
    },
    artifactTab: {
        borderRadius: "4px",
        opacity: 0.6,
        paddingBlock: "1px",
        paddingInline: "0.375rem",
    },
    artifactTabOn: {
        backgroundColor: chat.canvas,
        opacity: 1,
    },
    artifactPublish: {
        backgroundColor: chat.ink,
        borderRadius: "6px",
        color: chat.canvas,
        flexShrink: 0,
        fontSize: "0.6875rem",
        fontWeight: 600,
        paddingBlock: "0.1875rem",
        paddingInline: "0.5rem",
    },
    appRow: {
        alignItems: "center",
        borderBottomColor: chat.border,
        borderBottomStyle: "solid",
        borderBottomWidth: { default: "1px", ":last-of-type": 0 },
        display: "grid",
        gap: "0.5rem",
        gridTemplateColumns: "minmax(0, 7rem) minmax(0, 1fr) auto",
        paddingBlock: "0.375rem",
        paddingInline: "0.75rem",
    },
    appTitle: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    appMeta: {
        fontSize: "0.6875rem",
        opacity: 0.6,
        whiteSpace: "nowrap",
    },
    appAsked: {
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    appState: {
        fontSize: "0.6875rem",
        whiteSpace: "nowrap",
    },
    appShared: {
        backgroundColor: `color-mix(in srgb, ${assistant.passed} 8%, transparent)`,
        color: assistant.passed,
    },
    appFoot: {
        backgroundColor: `color-mix(in srgb, ${chat.ink} 5%, transparent)`,
        color: chat.ink,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        paddingBlock: "0.375rem",
        paddingInline: "0.75rem",
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
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
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
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
        transitionProperty: `${EASED}, opacity, transform`,
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
        minWidth: 0,
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
    grokComposer: {
        backgroundColor: grok.composer,
        borderColor: grok.border,
        borderRadius: "24px",
        borderStyle: "solid",
        borderWidth: "1px",
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
        gap: "0.5rem",
        paddingBlock: "0.1875rem",
        paddingInlineEnd: "0.5rem",
        paddingInlineStart: "0.25rem",
        whiteSpace: "nowrap",
    },
    logos: {
        display: "inline-flex",
        gap: "0.25rem",
    },
    logo: {
        alignItems: "center",
        backgroundColor: chat.canvas,
        borderColor: chat.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    idle: {
        opacity: 0.5,
    },
    placeholder: {
        color: chat.muted,
        fontSize: "0.875rem",
    },
    composerRow: {
        alignItems: "center",
        display: "flex",
        gap: "0.625rem",
    },
    hint: {
        alignItems: "center",
        color: chat.muted,
        display: { default: "inline-flex", [media.maxMd]: "none" },
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
    grokSend: {
        alignItems: "center",
        backgroundColor: grok.send,
        borderRadius: "50%",
        color: grok.sendInk,
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
