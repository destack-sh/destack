import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createSignal, type JSX } from "@destack/view";

import { installCommand } from "../content/site";
import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { systemOf, systems } from "./download/catalog";
import { createDownloads } from "./download/download";
import { Glyph } from "./glyph";

/** The media query for screens narrower than the desktop frame, where the ways stack. */
const narrow = "@media (max-width: 1099px)";

/** How long a copy key says it copied, in milliseconds. */
const copiedTime = 1600;

/** A way to install: its name, and its field drawn as the one control that installs. */
type Way = { name: string; field: () => JSX.Element };

/** Draw a field's key: the icon at its end that says what pressing the field does. */
function Key(properties: { isPrimary?: boolean; children: JSX.Element }) {
    return (
        <span
            aria-hidden="true"
            {...stylex.attrs(styles.key, properties.isPrimary === true && styles.keyPrimary)}
        >
            {properties.children}
        </span>
    );
}

/** Download the published build for this machine, or open the setup guide when none fits, from the whole field. */
function DownloadField() {
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
            title={`Destack for ${systems}`}
            {...stylex.attrs(styles.field, styles.tiled)}
        >
            <span {...stylex.attrs(styles.product)}>
                <img alt="" src="/brand/icon/icon-rounded.svg" {...stylex.attrs(styles.icon)} />
                Destack Desktop
            </span>
            <Key isPrimary>
                <Glyph name="download" size={16} />
            </Key>
        </a>
    );
}

/** Copy a text from the whole field, and show a check on its key for a moment. */
function CopyField(properties: {
    name: string;
    text: string;
    isDark?: boolean;
    children: JSX.Element;
}) {
    // hold whether the text was just copied
    const [isCopied, setIsCopied] = createSignal(false);

    return (
        <button
            type="button"
            aria-label={`Copy ${properties.name}`}
            title={isCopied() ? "Copied" : properties.text}
            onClick={() =>
                void navigator.clipboard.writeText(properties.text).then(() => {
                    setIsCopied(true);
                    setTimeout(() => setIsCopied(false), copiedTime);
                })
            }
            {...stylex.attrs(styles.field, properties.isDark === true && styles.dark)}
        >
            {properties.children}
            <Key>
                <Glyph name={isCopied() ? "check" : "copy"} size={16} />
            </Key>
        </button>
    );
}

/** Build the three ways to install, the agent prompt naming the tools you picked. */
function waysOf(prompt: string): readonly Way[] {
    return [
        { name: "Download Desktop", field: () => <DownloadField /> },
        {
            name: "Install via Terminal",
            field: () => (
                <CopyField name="command" text={installCommand} isDark>
                    <code {...stylex.attrs(styles.text)}>
                        <span {...stylex.attrs(styles.prompt)}>$ </span>
                        curl destack.sh/install | sh
                        <span {...stylex.attrs(styles.more)}>…</span>
                    </code>
                </CopyField>
            ),
        },
        {
            name: "Ask your Agent",
            field: () => (
                <CopyField name="prompt" text={prompt}>
                    <span {...stylex.attrs(styles.text)}>
                        Set up my Destack.
                        <span {...stylex.attrs(styles.more)}>…</span>
                    </span>
                </CopyField>
            ),
        },
    ];
}

/** Show the three ways to install side by side, each a name over the field that installs. */
export function InstallWays(properties: { prompt: string }) {
    return (
        <ol {...stylex.attrs(lattice.cell, styles.column)}>
            {waysOf(properties.prompt).map((way, index) => (
                <li {...stylex.attrs(styles.way)}>
                    <span {...stylex.attrs(styles.title)}>
                        {String(index + 1).padStart(2, "0")} {way.name}
                    </span>
                    {way.field()}
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
        backgroundColor: {
            default: color.card,
            ":hover": `color-mix(in srgb, ${color.card} 92%, ${tokens.signalInk})`,
        },
        borderColor: tokens.rule,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        boxSizing: "border-box",
        color: "inherit",
        cursor: "pointer",
        display: "flex",
        font: "inherit",
        gap: "0.75rem",
        height: "3rem",
        minWidth: 0,
        paddingInline: "1rem 0.375rem",
        textAlign: "start",
        textDecorationLine: "none",
        transition: "background-color 160ms ease, color 160ms ease, transform 120ms ease",
        width: "100%",
        ":active": { transform: "scale(0.99)" },
    },
    dark: {
        backgroundColor: {
            default: tokens.signalInk,
            ":hover": `color-mix(in srgb, ${tokens.signalInk} 88%, ${tokens.cream})`,
        },
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
        height: "2.25rem",
        width: "2.25rem",
    },
    tiled: {
        paddingInlineStart: "0.375rem",
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
        borderRadius: "6px",
        display: "inline-flex",
        flexShrink: 0,
        height: "2.25rem",
        justifyContent: "center",
        width: "2.25rem",
    },
    keyPrimary: {
        backgroundColor: tokens.signal,
        color: tokens.signalInk,
    },
});
