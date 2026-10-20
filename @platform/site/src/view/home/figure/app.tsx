import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, stroke } from "@destack/theme/tokens.stylex";
import type { JSX } from "@destack/view";

import { palette } from "../../palette.stylex";
import { AgentAvatar, isAgent } from "./agent";
import { paperGlyphOf, washOf } from "./planet";

/** A mock narrower than a phone screen, which then takes its phone layout. */
const NARROW = "@container (max-width: 479px)";

/** The type sizes of a drawn app of your space. */
export const appText = style.create({
    title: {
        fontSize: { default: "1.75rem", [NARROW]: "1.375rem" },
        fontWeight: 700,
        letterSpacing: "-0.02em",
        lineHeight: 1.15,
    },
    heading: {
        fontSize: "1rem",
        fontWeight: 700,
        lineHeight: 1.3,
    },
    body: {
        fontSize: "0.8125rem",
        lineHeight: 1.45,
    },
    meta: {
        fontSize: "0.75rem",
        lineHeight: 1.4,
    },
    small: {
        fontSize: "0.6875rem",
        lineHeight: 1.3,
    },
});

/** The shell of a drawn app of your space. */
export const appStyles = style.create({
    app: {
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: { default: "3rem minmax(0, 1fr)", [NARROW]: "minmax(0, 1fr)" },
        gridTemplateRows: { default: "minmax(0, 1fr) auto", [NARROW]: "minmax(0, 1fr) auto auto" },
        height: "100%",
        minHeight: 0,
    },
    rail: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRightColor: color.border,
        borderRightStyle: "solid",
        borderRightWidth: { default: stroke.border, [NARROW]: 0 },
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: { default: 0, [NARROW]: stroke.border },
        display: "flex",
        flexDirection: { default: "column", [NARROW]: "row" },
        gap: "0.25rem",
        gridRow: { default: "auto", [NARROW]: "3" },
        justifyContent: { default: "flex-start", [NARROW]: "space-around" },
        paddingBlock: { default: "0.75rem", [NARROW]: "0.375rem" },
    },
    railWide: {
        display: { default: "flex", [NARROW]: "none" },
    },
    railAvatar: {
        height: "1.625rem",
        marginBottom: "0.25rem",
        width: "1.625rem",
    },
    avatar: {
        alignItems: "center",
        borderRadius: "50%",
        color: "white",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "0.6rem",
        fontWeight: 700,
        height: "1.125rem",
        justifyContent: "center",
        width: "1.125rem",
    },
    railButton: {
        alignItems: "center",
        borderRadius: "6px",
        display: "flex",
        flexShrink: 0,
        height: "1.75rem",
        justifyContent: "center",
        position: "relative",
        width: "2rem",
    },
    railLink: {
        backgroundColor: {
            default: null,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in srgb, ${color.card} 55%, ${color.muted})`,
            },
        },
        cursor: "pointer",
    },
    glyph: {
        backgroundColor: color.mutedForeground,
        flexShrink: 0,
        height: "0.875rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "0.875rem",
    },
    railGlyph: {
        height: "1rem",
        width: "1rem",
    },
    glyphOn: {
        backgroundColor: color.primary,
    },
    badge: {
        backgroundColor: color.primary,
        borderRadius: "999px",
        color: color.primaryForeground,
        fontSize: "0.62rem",
        fontWeight: 700,
        paddingInline: "0.375rem",
    },
    railBadge: {
        fontSize: "0.55rem",
        paddingInline: "0.25rem",
        position: "absolute",
        right: "-0.125rem",
        top: "-0.125rem",
    },
    railRule: {
        display: { default: "block", [NARROW]: "none" },
        backgroundColor: color.border,
        flexShrink: 0,
        height: stroke.border,
        marginBlock: "0.125rem",
        width: "1.5rem",
    },
    main: {
        display: "grid",
        minHeight: 0,
        minWidth: 0,
    },
    page: {
        alignContent: "start",
        display: "grid",
        gap: "1.125rem",
        gridAutoRows: "max-content",
        gridTemplateColumns: "minmax(0, 1fr)",
        maskImage: "linear-gradient(to bottom, black calc(100% - 3rem), transparent)",
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: "1.5rem",
        paddingInline: { default: "2rem", [NARROW]: "1rem" },
    },
    toolbar: {
        alignItems: "center",
        display: "flex",
        flexWrap: "wrap",
        rowGap: "0.5rem",
        columnGap: "1rem",
        margin: 0,
    },
    tools: {
        alignItems: "center",
        display: "flex",
        gap: "1.25rem",
        marginLeft: "auto",
    },
    action: {
        color: color.mutedForeground,
        display: "block",
        lineHeight: "1.25rem",
        whiteSpace: "nowrap",
    },
    actionStrong: {
        color: color.foreground,
        fontWeight: 600,
    },
    primary: {
        backgroundColor: color.foreground,
        borderRadius: "5px",
        color: color.background,
        fontWeight: 600,
        paddingBlock: "0.125rem",
        paddingInline: "0.625rem",
        whiteSpace: "nowrap",
    },
    property: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "5px",
        boxSizing: "border-box",
        color: color.mutedForeground,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.375rem",
        height: "1.375rem",
        lineHeight: 1,
        paddingInline: "0.5rem",
        verticalAlign: "middle",
        whiteSpace: "nowrap",
    },
    properties: {
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem",
    },
    statusBar: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        boxSizing: "border-box",
        color: color.mutedForeground,
        display: "flex",
        fontSize: "0.75rem",
        gap: "1.25rem",
        gridColumn: "1 / -1",
        height: "2.25rem",
        margin: 0,
        paddingInline: "0.875rem",
        whiteSpace: "nowrap",
    },
    status: {
        alignItems: "center",
        display: "flex",
        gap: "0.375rem",
    },
    statusEnd: {
        alignItems: "center",
        display: { default: "flex", [NARROW]: "none" },
        gap: "1.25rem",
        marginLeft: "auto",
    },
});

/** Draw a glyph of the diagram set, such as an app's mark in the rail, tinted when lit. */
export function AppGlyph(properties: { name: string; isOn?: boolean }) {
    return (
        <span
            aria-hidden="true"
            style={{ "mask-image": `url(/diagram/${properties.name}.svg)` }}
            {...style.attrs(appStyles.glyph, properties.isOn === true && appStyles.glyphOn)}
        />
    );
}

/** Draw the rail every app of your space shares, its apps clickable given an open handler. */
export function AppRail(properties: {
    open: string;
    apps: readonly (readonly [name: string, label: string])[];
    morph: string | undefined;
    onOpen?: (name: string) => void;
}) {
    return (
        <nav data-component="AppRail" {...style.attrs(appStyles.rail)}>
            <span
                title="Your space"
                style={{ "background-color": palette.teal }}
                {...style.attrs(appStyles.avatar, appStyles.railAvatar, appStyles.railWide)}
            >
                F
            </span>
            <span title="Search" {...style.attrs(appStyles.railButton, appStyles.railWide)}>
                <AppGlyph name="search" />
            </span>
            <span title="Inbox" {...style.attrs(appStyles.railButton, appStyles.railWide)}>
                <AppGlyph name="notify" />
                <span {...style.attrs(appStyles.badge, appStyles.railBadge)}>3</span>
            </span>
            <span aria-hidden="true" {...style.attrs(appStyles.railRule)} />
            {properties.apps.map(([name, label]) => (
                <span
                    data-component="AppLink"
                    data-morph-icon={properties.morph === undefined ? undefined : ""}
                    title={label}
                    onClick={() => properties.onOpen?.(name)}
                    style={{
                        "background-color": name === properties.open ? washOf(name) : undefined,
                        "view-transition-name":
                            properties.morph === undefined
                                ? undefined
                                : `${properties.morph}-icon-${name}`,
                    }}
                    {...style.attrs(
                        appStyles.railButton,
                        properties.onOpen !== undefined && appStyles.railLink,
                    )}
                >
                    {isAgent(name) ? (
                        <AgentAvatar size="glyph" />
                    ) : (
                        <span
                            aria-hidden="true"
                            style={{
                                "mask-image": `url(/diagram/${name}.svg)`,
                                "background-color":
                                    name === properties.open ? paperGlyphOf(name) : undefined,
                            }}
                            {...style.attrs(appStyles.glyph, appStyles.railGlyph)}
                        />
                    )}
                </span>
            ))}
        </nav>
    );
}

/** Draw a page's header. */
export function AppHeader(properties: { title: JSX.Element; children?: JSX.Element }) {
    return (
        <div data-component="PageHeader" {...style.attrs(appStyles.toolbar)}>
            <b data-component="PageTitle" {...style.attrs(appText.title)}>
                {properties.title}
            </b>
            <div {...style.attrs(appStyles.tools)}>{properties.children}</div>
        </div>
    );
}
