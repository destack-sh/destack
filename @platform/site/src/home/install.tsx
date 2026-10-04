import * as stylex from "@destack/style";
import { createSignal, type JSX } from "@destack/view";

import { agentPrompt, installCommand } from "../content/site";
import { paper } from "../style/paper.stylex";
import { tokens } from "../style/tokens.stylex";
import { systemOf, systems } from "./download/catalog";
import { createDownloads } from "./download/download";
import { Caption, Window } from "./window";

/** The media query for screens narrower than the desktop frame, where the frames stack. */
const narrow = "@media (max-width: 1099px)";

/** How long a copy key says it copied, in milliseconds. */
const copiedTime = 1600;

/** The terminal session the install frame shows: commands and their output. */
const terminalLines: readonly (readonly ["command" | "output", string])[] = [
    ["command", installCommand],
    ["output", "Destack installed in ~/.destack"],
    ["command", "destack login"],
    ["output", "Signed in as ada"],
];

/** The muted ink inside the cream windows. */
const quietInk = "#5d7076";
/** The muted ink of command output on the terminal's dark ground. */
const outputInk = "#8aa3ab";

/** Download the published build for this machine, or open the setup guide when none fits. */
function DownloadKey() {
    // hold the build chosen for this machine
    const { selected } = createDownloads();
    const system = () => {
        const download = selected();

        return download === undefined ? undefined : systemOf(download.label);
    };

    return (
        <a
            href={selected()?.url ?? "/docs/setup/"}
            title={selected() === undefined ? undefined : `Version ${selected()?.version}`}
            {...stylex.attrs(paper.key, paper.primary)}
        >
            {system() === undefined ? "Download" : `Download for ${system()}`}
        </a>
    );
}

/** Copy a text, and say so on the key for a moment. */
function CopyKey(properties: { text: string }) {
    // hold whether the text was just copied
    const [isCopied, setIsCopied] = createSignal(false);

    return (
        <button
            type="button"
            onClick={() =>
                void navigator.clipboard.writeText(properties.text).then(() => {
                    setIsCopied(true);
                    setTimeout(() => setIsCopied(false), copiedTime);
                })
            }
            {...stylex.attrs(paper.key, paper.small)}
        >
            {isCopied() ? "Copied" : "Copy"}
        </button>
    );
}

/** Frame one way to install: a window holding its control, and a caption under it. */
function Frame(properties: { verb: string; rest: string; title: string; children: JSX.Element }) {
    return (
        <article {...stylex.attrs(styles.frame)}>
            <Window title={properties.title}>{properties.children}</Window>
            <Caption verb={properties.verb} rest={properties.rest} />
        </article>
    );
}

/** Frame the three ways to install: the desktop app, the terminal and an agent. */
export function InstallFrames() {
    return (
        <>
            <Frame verb="Download" rest=" the app" title="Destack">
                <div {...stylex.attrs(styles.desktop)}>
                    <DownloadKey />
                    <span {...stylex.attrs(styles.quiet)}>{systems}</span>
                </div>
            </Frame>
            <Frame verb="Paste" rest=" one line" title="Terminal">
                <div {...stylex.attrs(styles.terminal)}>
                    <pre {...stylex.attrs(styles.lines)}>
                        {terminalLines.map(([kind, line]) => (
                            <span
                                {...stylex.attrs(styles.line, kind === "output" && styles.output)}
                            >
                                {kind === "command" && (
                                    <span {...stylex.attrs(styles.prompt)}>$ </span>
                                )}
                                {line}
                            </span>
                        ))}
                    </pre>
                    <CopyKey text={installCommand} />
                </div>
            </Frame>
            <Frame verb="Ask" rest=" your agent" title="Agent">
                <div {...stylex.attrs(styles.chat)}>
                    <p {...stylex.attrs(styles.bubble, styles.mine)}>{agentPrompt}</p>
                    <p {...stylex.attrs(styles.bubble, styles.theirs)}>
                        Reading the setup guide and installing Destack.
                    </p>
                    <CopyKey text={agentPrompt} />
                </div>
            </Frame>
        </>
    );
}

/** The install frame styles. */
const styles = stylex.create({
    frame: {
        alignContent: "center",
        borderBottomWidth: 0,
        borderColor: tokens.rule,
        borderLeftWidth: 0,
        borderRightWidth: { default: tokens.hairline, ":nth-of-type(3n)": 0, [narrow]: 0 },
        borderStyle: "solid",
        borderTopWidth: 0,
        display: "grid",
        gap: "1.125rem",
        gridColumn: "span 4",
        gridTemplateRows: `calc(${tokens.stage} * 2.5 - 7rem) auto`,
        minWidth: 0,
        paddingBottom: "3rem",
        paddingInline: tokens.inset,
        [narrow]: { gridColumn: "1 / -1", paddingBottom: "2rem" },
    },
    desktop: {
        alignContent: "center",
        display: "grid",
        gap: "0.75rem",
        justifyItems: "center",
    },
    quiet: {
        color: quietInk,
        fontFamily: tokens.monoFont,
        fontSize: "0.72rem",
    },
    terminal: {
        alignContent: "space-between",
        backgroundColor: tokens.signalInk,
        display: "grid",
        justifyItems: "start",
        padding: "0.875rem",
    },
    lines: {
        color: tokens.cream,
        fontFamily: tokens.monoFont,
        fontSize: "0.74rem",
        lineHeight: 1.7,
        margin: 0,
        overflowWrap: "anywhere",
        whiteSpace: "pre-wrap",
    },
    line: {
        display: "block",
    },
    prompt: {
        color: tokens.signal,
    },
    output: {
        color: outputInk,
    },
    chat: {
        alignContent: "end",
        display: "grid",
        fontSize: "0.88rem",
        gap: "0.5rem",
        justifyItems: "start",
        padding: "0.75rem",
    },
    bubble: {
        lineHeight: 1.35,
        margin: 0,
        maxWidth: "88%",
        overflowWrap: "anywhere",
        paddingBlock: "0.5rem",
        paddingInline: "0.625rem",
    },
    mine: {
        backgroundColor: tokens.space,
        color: tokens.cream,
        justifySelf: "end",
    },
    theirs: {
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "1.5px",
        color: quietInk,
    },
});
