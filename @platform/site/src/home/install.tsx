import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createSignal, type JSX } from "@destack/view";

import { installCommand } from "../content/site";
import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { systemOf, systems } from "./download/catalog";
import { createDownloads } from "./download/download";
import { Mark } from "../site/mark";
import { Glyph } from "./glyph";

/** The media query for screens narrower than the desktop frame, where the ways stack. */
const narrow = "@media (max-width: 1099px)";

/** How long a copy key says it copied, in milliseconds. */
const copiedTime = 1600;

/** A way to install: its name, what its field shows, and its key. */
type Way = { name: string; field: () => JSX.Element; key: () => JSX.Element; isDark?: boolean };

/** Download the published build for this machine, or open the setup guide when none fits, as an icon key. */
function DownloadKey() {
    // hold the build chosen for this machine
    const { selected } = createDownloads();
    const system = () => {
        const download = selected();

        return download === undefined ? undefined : systemOf(download.label);
    };
    const label = () => (system() === undefined ? "Download" : `Download for ${system()}`);

    return (
        <a
            href={selected()?.url ?? "/docs/setup/"}
            aria-label={label()}
            title={label()}
            {...stylex.attrs(styles.key, styles.keyPrimary)}
        >
            <Glyph name="download" size={16} />
        </a>
    );
}

/** Copy a text, and show a check on the key for a moment. */
function CopyKey(properties: { name: string; text: string }) {
    // hold whether the text was just copied
    const [isCopied, setIsCopied] = createSignal(false);

    return (
        <button
            type="button"
            aria-label={`Copy ${properties.name}`}
            title={isCopied() ? "Copied" : `Copy ${properties.name}`}
            onClick={() =>
                void navigator.clipboard.writeText(properties.text).then(() => {
                    setIsCopied(true);
                    setTimeout(() => setIsCopied(false), copiedTime);
                })
            }
            {...stylex.attrs(styles.key)}
        >
            <Glyph name={isCopied() ? "check" : "copy"} size={16} />
        </button>
    );
}

/** Build the three ways to install, the agent prompt naming the tools you picked. */
function waysOf(prompt: string): readonly Way[] {
    return [
        {
            name: "Download Desktop",
            field: () => (
                <span title={`Destack for ${systems}`} {...stylex.attrs(styles.product)}>
                    <Mark style={styles.icon} />
                    Destack Desktop
                </span>
            ),
            key: () => <DownloadKey />,
        },
        {
            name: "Install via Terminal",
            field: () => (
                <code title={installCommand} {...stylex.attrs(styles.text)}>
                    <span {...stylex.attrs(styles.prompt)}>$ </span>
                    curl destack.sh/install | sh
                    <span {...stylex.attrs(styles.more)}>…</span>
                </code>
            ),
            key: () => <CopyKey name="command" text={installCommand} />,
            isDark: true,
        },
        {
            name: "Ask your Agent",
            field: () => (
                <span title={prompt} {...stylex.attrs(styles.text)}>
                    Set up my Destack.
                    <span {...stylex.attrs(styles.more)}>…</span>
                </span>
            ),
            key: () => <CopyKey name="prompt" text={prompt} />,
        },
    ];
}

/** Show the three ways to install side by side, each a name over one field with its key. */
export function InstallWays(properties: { prompt: string }) {
    return (
        <ol {...stylex.attrs(lattice.cell, styles.column)}>
            {waysOf(properties.prompt).map((way, index) => (
                <li {...stylex.attrs(styles.way)}>
                    <span {...stylex.attrs(styles.title)}>
                        {String(index + 1).padStart(2, "0")} {way.name}
                    </span>
                    <span {...stylex.attrs(styles.field, way.isDark === true && styles.dark)}>
                        {way.field()}
                        {way.key()}
                    </span>
                </li>
            ))}
        </ol>
    );
}

/** The install styles. */
const styles = stylex.create({
    column: {
        columnGap: "1.5rem",
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: {
            default: "repeat(3, minmax(0, 1fr))",
            [narrow]: "minmax(0, 1fr)",
        },
        listStyle: "none",
        margin: 0,
        padding: `1.5rem ${tokens.inset}`,
        rowGap: "1rem",
    },
    way: {
        display: "grid",
        gap: "0.5rem",
        minWidth: 0,
    },
    title: {
        color: tokens.signal,
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        textTransform: "uppercase",
    },
    field: {
        alignItems: "center",
        backgroundColor: color.card,
        borderColor: tokens.rule,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "flex",
        gap: "0.75rem",
        height: "3rem",
        minWidth: 0,
        paddingInline: "1rem 0.375rem",
        transition: "background-color 160ms ease, color 160ms ease",
    },
    dark: {
        backgroundColor: tokens.signalInk,
        borderColor: tokens.signalInk,
        color: tokens.cream,
    },
    product: {
        alignItems: "center",
        display: "flex",
        flexGrow: 1,
        fontSize: "0.875rem",
        fontWeight: 600,
        gap: "0.625rem",
        minWidth: 0,
        whiteSpace: "nowrap",
    },
    icon: {
        flexShrink: 0,
        height: "1.5rem",
        width: "1.5rem",
    },
    text: {
        flexGrow: 1,
        fontFamily: tokens.monoFont,
        fontSize: "0.8125rem",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    prompt: {
        color: tokens.signal,
    },
    more: {
        borderRadius: "4px",
        cursor: "help",
        marginLeft: "0.5rem",
        opacity: 0.55,
        paddingInline: "0.25rem",
        ":hover": { opacity: 1 },
    },
    key: {
        alignItems: "center",
        backgroundColor: { default: "transparent", ":hover": "rgb(127 127 127 / 14%)" },
        borderRadius: "6px",
        borderWidth: 0,
        color: "inherit",
        cursor: "pointer",
        display: "inline-flex",
        flexShrink: 0,
        height: "2.25rem",
        justifyContent: "center",
        transition: "background-color 120ms ease, transform 120ms ease",
        width: "2.25rem",
        ":active": { transform: "scale(0.94)" },
    },
    keyPrimary: {
        backgroundColor: { default: tokens.signal, ":hover": tokens.signal },
        color: tokens.signalInk,
    },
});
