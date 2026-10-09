import { frame } from "../../layout/frame.stylex";
import { media } from "@destack/style/media.stylex";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { installCommand } from "../../content/site";
import { lattice } from "../../layout/lattice.stylex";
import { systemOf, systems } from "../participle/download/catalog";
import { createDownloads } from "../participle/download/download";
import { Glyph } from "../figure/glyph";
import { screen } from "../../layout/screen.stylex";
import { createCopy } from "../../layout/copy";

/** A way to install: its name, and its field drawn as the one control that installs. */
type Way = { name: string; field: () => JSX.Element };

/** Draw a field's key: the icon at its end that says what pressing the field does. */
function Key(properties: { isPrimary?: boolean; children: JSX.Element }) {
    return (
        <span
            aria-hidden="true"
            {...style.attrs(styles.key, properties.isPrimary === true && styles.keyPrimary)}
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
            {...style.attrs(styles.field, styles.tiled)}
        >
            <span {...style.attrs(styles.product)}>
                <img alt="" src="/brand/icon/icon-rounded.svg" {...style.attrs(styles.icon)} />
                Destack Desktop
            </span>
            <Key isPrimary>
                <Glyph name="download" size={16} />
            </Key>
        </a>
    );
}

/** Copy a text from the whole field, and show a check on its key for a moment. */
function CopyField(properties: { name: string; text: string; children: JSX.Element }) {
    // copy the text and show it as copied for a moment
    const copied = createCopy();

    return (
        <button
            type="button"
            aria-label={`Copy ${properties.name}`}
            title={copied.isCopied() ? "Copied" : properties.text}
            onClick={() => void copied.copy(properties.text)}
            {...style.attrs(styles.field)}
        >
            {properties.children}
            <Key>
                <Glyph name={copied.isCopied() ? "check" : "copy"} size={16} />
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
                <CopyField name="command" text={installCommand}>
                    <code {...style.attrs(styles.text)}>
                        <span {...style.attrs(styles.prompt)}>$ </span>
                        curl destack.sh/install | sh
                        <span {...style.attrs(styles.more)}>…</span>
                    </code>
                </CopyField>
            ),
        },
        {
            name: "Ask your Agent",
            field: () => (
                <CopyField name="prompt" text={prompt}>
                    <span {...style.attrs(styles.text)}>
                        Set up my Destack.
                        <span {...style.attrs(styles.more)}>…</span>
                    </span>
                </CopyField>
            ),
        },
    ];
}

/** Show the three ways to install side by side in ruled cells, each a numbered name over the field that installs. */
export function InstallWays(properties: { prompt: string }) {
    return (
        <ol {...style.attrs(lattice.cell, styles.column)}>
            {waysOf(properties.prompt).map((way, index) => (
                <li {...style.attrs(styles.way)}>
                    <span {...style.attrs(styles.title)}>
                        <span {...style.attrs(styles.number)}>
                            {String(index + 1).padStart(2, "0")}
                        </span>
                        {way.name}
                    </span>
                    {way.field()}
                </li>
            ))}
        </ol>
    );
}

/** The install styles. */
const styles = style.create({
    column: {
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: {
            default: "repeat(3, minmax(0, 1fr))",
            [screen.belowDesktop]: "minmax(0, 1fr)",
        },
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    way: {
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: 0,
        borderInlineStartWidth: {
            default: 0,
            ":not(:first-child)": { default: stroke.border, [screen.belowDesktop]: 0 },
        },
        borderTopWidth: {
            default: 0,
            ":not(:first-child)": { default: 0, [screen.belowDesktop]: stroke.border },
        },
        display: "grid",
        gap: "0.75rem",
        minWidth: 0,
        paddingBottom: "1.5rem",
        paddingInline: frame.inset,
        paddingTop: "1.25rem",
    },
    title: {
        alignItems: "baseline",
        display: "flex",
        fontSize: "0.875rem",
        fontWeight: 600,
    },
    number: {
        color: color.primary,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        marginRight: "1rem",
    },
    field: {
        alignItems: "center",
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
        },
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        boxSizing: "border-box",
        color: "inherit",
        cursor: "pointer",
        display: "flex",
        font: "inherit",
        gap: "0.75rem",
        height: "3rem",
        minWidth: 0,
        paddingInlineStart: "1rem",
        paddingInlineEnd: "0.375rem",
        textAlign: "start",
        textDecorationLine: "none",
        transition: "background-color 160ms ease, color 160ms ease, transform 120ms ease",
        width: "100%",
        ":active": { transform: "scale(0.99)" },
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
        fontFamily: font.code,
        fontSize: "0.8125rem",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    prompt: {
        color: color.primary,
    },
    more: {
        borderRadius: "4px",
        cursor: "help",
        marginLeft: "0.5rem",
        opacity: { default: 0.55, ":hover": { default: null, [media.hover]: 1 } },
        paddingInline: "0.25rem",
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
        backgroundColor: color.primary,
        color: color.primaryForeground,
    },
});
