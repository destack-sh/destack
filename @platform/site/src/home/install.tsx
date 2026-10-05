import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createSignal, type JSX } from "@destack/view";

import { agentPrompt, installCommand } from "../content/site";
import { lattice } from "../style/lattice.stylex";
import { plate } from "../style/plate.stylex";
import { paper } from "../style/paper.stylex";
import { tokens } from "../style/tokens.stylex";
import { systemOf, systems } from "./download/catalog";
import { createDownloads } from "./download/download";
import { Mark } from "../site/mark";

/** The media query for screens narrower than the desktop frame, where the ways stack. */
const narrow = "@media (max-width: 1099px)";

/** How long a copy key says it copied, in milliseconds. */
const copiedTime = 1600;

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
function CopyKey(properties: { name: string; text: string }) {
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
            {...stylex.attrs(paper.key)}
        >
            {isCopied() ? "Copied" : `Copy ${properties.name}`}
        </button>
    );
}

/** The three ways to install, set as the figures set their ledgers: a label, a claim with its plates, a line under it, what it shows, and its key. */
const ways: readonly {
    label: string;
    claim: string;
    plates: readonly string[];
    note: string;
    body: () => JSX.Element;
    key: () => JSX.Element;
}[] = [
    {
        label: "Desktop",
        claim: "Download the app",
        plates: ["macOS", "Linux"],
        note: "your apps and data on this machine",
        body: () => (
            <span {...stylex.attrs(styles.download)}>
                <Mark style={styles.icon} />
                <span {...stylex.attrs(styles.product)}>
                    <b {...stylex.attrs(styles.productName)}>Destack</b>
                    <span {...stylex.attrs(styles.productNote)}>for {systems}</span>
                </span>
            </span>
        ),
        key: () => <DownloadKey />,
    },
    {
        label: "Terminal",
        claim: "Run one command",
        plates: ["macOS", "Linux"],
        note: "installs into ~/.destack",
        body: () => (
            <code {...stylex.attrs(styles.code, styles.dark)}>
                <span>
                    <span {...stylex.attrs(styles.prompt)}>$ </span>
                    {installCommand}
                </span>
            </code>
        ),
        key: () => <CopyKey name="command" text={installCommand} />,
    },
    {
        label: "Agent",
        claim: "Ask your agent",
        plates: ["one prompt"],
        note: "it follows the setup guide",
        body: () => <code {...stylex.attrs(styles.code)}>{agentPrompt}</code>,
        key: () => <CopyKey name="prompt" text={agentPrompt} />,
    },
];

/** Show the three ways to install side by side, each with its key at the foot. */
export function InstallWays() {
    return (
        <>
            {ways.map((way, index) => (
                <article {...stylex.attrs(lattice.cell, styles.way)}>
                    <span {...stylex.attrs(styles.label)}>
                        {String(index + 1).padStart(2, "0")} {way.label}
                    </span>
                    <span {...stylex.attrs(styles.claim)}>
                        <b {...stylex.attrs(styles.name)}>{way.claim}</b>
                        <span {...stylex.attrs(styles.plates)}>
                            {way.plates.map((name) => (
                                <span {...stylex.attrs(plate.plate)}>{name}</span>
                            ))}
                        </span>
                    </span>
                    <span {...stylex.attrs(styles.note)}>{way.note}</span>
                    {way.body()}
                </article>
            ))}
            {ways.map((way) => (
                <div {...stylex.attrs(lattice.cell, styles.key)}>{way.key()}</div>
            ))}
        </>
    );
}

/** The install styles. */
const styles = stylex.create({
    way: {
        alignContent: "start",
        display: "grid",
        gridColumn: "span 4",
        gridTemplateRows: "auto auto auto minmax(0, 1fr)",
        padding: `1.5rem ${tokens.inset}`,
        rowGap: "0.25rem",
        [narrow]: { gridColumn: "1 / -1" },
    },
    key: {
        display: "grid",
        gridColumn: "span 4",
        padding: `1rem ${tokens.inset}`,
        [narrow]: { gridColumn: "1 / -1" },
    },
    label: {
        color: tokens.signal,
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        lineHeight: "1rem",
        textTransform: "uppercase",
    },
    claim: {
        alignItems: "center",
        display: "flex",
        gap: "0.5rem",
    },
    name: {
        fontSize: "1rem",
        fontWeight: 600,
        lineHeight: "1.375rem",
    },
    plates: {
        display: "flex",
        gap: "0.25rem",
        marginLeft: "auto",
    },
    note: {
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        letterSpacing: "0.02em",
        lineHeight: "1.125rem",
        marginBottom: "1rem",
        opacity: 0.8,
    },
    download: {
        alignItems: "center",
        backgroundColor: color.card,
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        gap: "0.875rem",
        paddingBlock: "0.75rem",
        paddingInline: "0.875rem",
    },
    icon: {
        height: "2.5rem",
        width: "2.5rem",
    },
    product: {
        display: "grid",
    },
    productName: {
        fontSize: "1rem",
        fontWeight: 700,
    },
    productNote: {
        color: color.mutedForeground,
        fontSize: "0.8125rem",
    },
    code: {
        alignItems: "center",
        display: "flex",
        backgroundColor: color.card,
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: color.cardForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        lineHeight: 1.6,
        overflowWrap: "anywhere",
        paddingBlock: "0.75rem",
        paddingInline: "0.875rem",
    },
    dark: {
        backgroundColor: tokens.signalInk,
        borderColor: tokens.signalInk,
        color: tokens.cream,
    },
    prompt: {
        color: tokens.signal,
    },
});
