import { frame } from "../layout/frame.stylex";
import { media } from "@destack/style/media.stylex";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { type JSX, Show } from "@destack/view";

import { installCommand } from "../content/site";
import { lattice } from "../layout/lattice.stylex";
import { press } from "../layout/press";
import { type Download, type System, systems } from "./catalog";
import { createDownloads } from "./download";
import { Glyph } from "../home/figure/glyph";
import { screen } from "../layout/screen.stylex";
import { createCopy } from "../layout/copy";
import { tracking, typeScale } from "../layout/type.stylex";
import { brand } from "../brand";

/** The detected system and published build the download field shows. */
type Desktop = Pick<ReturnType<typeof createDownloads>, "system" | "selected">;

/** A way to install: its name, and its field drawn as the one control that installs. */
type Way = { name: () => string; field: () => JSX.Element };

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

/** Download the published build for this machine, or say why none fits. */
function DownloadField(properties: { system: System | undefined; download: Download | undefined }) {
    // explain a missing build: none for this system yet, or none for phones and tablets
    const missing = () =>
        properties.system === undefined
            ? `Destack Desktop runs on ${systems}`
            : `No ${properties.system} build yet`;

    return (
        <Show
            when={properties.download}
            fallback={
                <span
                    aria-disabled="true"
                    title={missing()}
                    {...style.attrs(press.key, styles.field, styles.tiled, styles.unavailable)}
                >
                    <Product />
                    <span {...style.attrs(styles.soon)}>
                        {properties.system === undefined ? "Desktop" : "Soon"}
                    </span>
                </span>
            }
        >
            {(download) => (
                <a
                    href={download().url}
                    aria-label={`Download ${download().label}`}
                    title={`Destack for ${download().label}`}
                    {...style.attrs(press.key, styles.field, styles.tiled)}
                >
                    <Product />
                    <Key isPrimary>
                        <Glyph name="download" size={16} />
                    </Key>
                </a>
            )}
        </Show>
    );
}

/** Draw the desktop app's icon and name. */
function Product() {
    return (
        <span {...style.attrs(styles.product)}>
            <img alt="" src={brand.iconRounded} {...style.attrs(styles.icon)} />
            Destack Desktop
        </span>
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
            {...style.attrs(press.key, styles.field)}
        >
            {properties.children}
            <Key>
                <Glyph name={copied.isCopied() ? "check" : "copy"} size={16} />
            </Key>
        </button>
    );
}

/** Offer the agent prompt as one field that copies itself. */
function AgentField(properties: { prompt: string }) {
    return (
        <CopyField name="prompt" text={properties.prompt}>
            <span {...style.attrs(styles.text)}>
                <span {...style.attrs(styles.prompt)}>&gt; </span>
                {properties.prompt}
            </span>
        </CopyField>
    );
}

/** Offer the install command as one field that copies itself. */
function TerminalField() {
    return (
        <CopyField name="command" text={installCommand}>
            <code {...style.attrs(styles.text)}>
                <span {...style.attrs(styles.prompt)}>$ </span>
                {installCommand}
            </code>
        </CopyField>
    );
}

/** Build the three ways to install, the agent prompt listing the tools you picked. */
function waysOf(prompt: string, desktop: Desktop): readonly Way[] {
    return [
        {
            name: () => "Ask your Agent",
            field: () => <AgentField prompt={prompt} />,
        },
        {
            name: () => {
                const system = desktop.system();

                return system === undefined ? "Download Desktop" : `Download for ${system}`;
            },
            field: () => <DownloadField system={desktop.system()} download={desktop.selected()} />,
        },
        {
            name: () => "Install via Terminal",
            field: () => <TerminalField />,
        },
    ];
}

/** Show the three ways to install side by side as alternatives, any one of them enough. */
export function InstallWays(properties: { prompt: string }) {
    // detect this machine's system and its published build once for the download field
    const desktop = createDownloads();

    return (
        <ul {...style.attrs(lattice.cell, styles.column)}>
            {waysOf(properties.prompt, desktop).map((way, index) => (
                <li {...style.attrs(styles.way)}>
                    {/* lead every way after the first with an "or" */}
                    <span {...style.attrs(styles.title)}>
                        {index === 0 ? undefined : (
                            <span aria-hidden="true" {...style.attrs(styles.or)}>
                                or
                            </span>
                        )}
                        {way.name()}
                    </span>
                    {way.field()}
                </li>
            ))}
        </ul>
    );
}

/** Show the three ways to install stacked, for the menu every page opens from its top bar. */
export function InstallList(properties: { prompt: string; xstyle?: style.Styles }) {
    // detect this machine's system and its published build once for the download field
    const desktop = createDownloads();

    return (
        <ul {...style.attrs(styles.list, properties.xstyle)}>
            {waysOf(properties.prompt, desktop).map((way) => (
                <li {...style.attrs(styles.item)}>
                    <span {...style.attrs(styles.title)}>{way.name()}</span>
                    {way.field()}
                </li>
            ))}
        </ul>
    );
}

/** The install styles. */
const styles = style.create({
    column: {
        backgroundColor: color.primary,
        backgroundImage: "none",
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
        borderColor: `color-mix(in srgb, ${color.foreground} 30%, transparent)`,
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
        paddingInline: { default: "2.25rem", [screen.belowDesktop]: frame.inset },
        paddingTop: "1.25rem",
        position: "relative",
    },
    title: {
        alignItems: "baseline",
        display: "flex",
        fontSize: typeScale.body,
        fontWeight: 600,
    },
    or: {
        color: `color-mix(in srgb, ${color.foreground} 70%, transparent)`,
        fontFamily: font.text,
        fontSize: typeScale.small,
        fontStyle: "italic",
        fontWeight: 400,
        marginInlineEnd: "0.375rem",
    },
    field: {
        alignItems: "center",
        backgroundColor: {
            default: color.background,
            ":hover": { default: null, [media.hover]: color.muted },
        },
        borderColor: color.foreground,
        borderRadius: 0,
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
        width: "100%",
    },
    list: {
        display: "grid",
        listStyle: "none",
        margin: 0,
        padding: 0,
        rowGap: "1rem",
        width: "min(22rem, calc(100vw - 2rem))",
    },
    item: {
        display: "grid",
        gap: "0.5rem",
    },
    unavailable: {
        backgroundColor: "transparent",
        boxShadow: "none",
        borderStyle: "dashed",
        cursor: "default",
        transform: "none",
    },
    soon: {
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: 600,
        letterSpacing: tracking.label,
        paddingInline: "0.625rem",
        textTransform: "uppercase",
    },
    product: {
        alignItems: "center",
        display: "flex",
        flexGrow: 1,
        fontSize: typeScale.small,
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
        fontSize: { default: typeScale.small, [media.maxSm]: typeScale.label },
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    prompt: {
        color: color.primary,
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
