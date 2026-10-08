import * as style from "@destack/style";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import type { JSX } from "@destack/view";

import { palette } from "../../palette.stylex";

/** The type sizes of a drawn app of your space: its page title, its body, its meta and its smallest labels. */
export const appText = style.create({
    title: {
        fontSize: { default: "1.75rem", "@media (max-width: 767px)": "1.375rem" },
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

/** The shell of a drawn app of your space: the rail of apps, the page with its header, and the status bar. */
export const appStyles = style.create({
    app: {
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: "3rem minmax(0, 1fr)",
        gridTemplateRows: "minmax(0, 1fr) auto",
        height: "100%",
        minHeight: 0,
        "@media (max-width: 767px)": { gridTemplateColumns: "minmax(0, 1fr)" },
    },
    rail: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRightColor: color.border,
        borderRightStyle: "solid",
        borderRightWidth: stroke.border,
        display: "flex",
        flexDirection: "column",
        gap: "0.5rem",
        paddingBlock: "0.75rem",
        "@media (max-width: 767px)": { display: "none" },
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
        height: "2rem",
        justifyContent: "center",
        position: "relative",
        width: "2rem",
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
    glyphOn: {
        backgroundColor: color.primary,
    },
    selected: {
        backgroundColor: `color-mix(in srgb, ${color.primary} 14%, transparent)`,
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
        paddingInline: { default: "2rem", "@media (max-width: 767px)": "1rem" },
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
        color: color.mutedForeground,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.375rem",
        paddingBlock: "0.125rem",
        paddingInline: "0.5rem",
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
        display: "flex",
        gap: "1.25rem",
        marginLeft: "auto",
    },
    dot: {
        backgroundColor: palette.lightGreen,
        borderRadius: "50%",
        height: "0.5rem",
        width: "0.5rem",
    },
    code: {
        fontFamily: font.code,
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

/** Draw the rail every app of your space shares: your account, search, the inbox, and your apps with the open one lit. */
export function AppRail(properties: {
    open: string;
    apps: readonly (readonly [name: string, label: string])[];
}) {
    return (
        <nav data-component="AppRail" {...style.attrs(appStyles.rail)}>
            <span
                title="Your space"
                style={{ "background-color": palette.teal }}
                {...style.attrs(appStyles.avatar, appStyles.railAvatar)}
            >
                F
            </span>
            <span title="Search" {...style.attrs(appStyles.railButton)}>
                <AppGlyph name="search" />
            </span>
            <span title="Inbox" {...style.attrs(appStyles.railButton)}>
                <AppGlyph name="notify" />
                <span {...style.attrs(appStyles.badge, appStyles.railBadge)}>3</span>
            </span>
            <span aria-hidden="true" {...style.attrs(appStyles.railRule)} />
            {properties.apps.map(([name, label]) => (
                <span
                    data-component="AppLink"
                    title={label}
                    {...style.attrs(
                        appStyles.railButton,
                        name === properties.open && appStyles.selected,
                    )}
                >
                    <AppGlyph name={name} isOn={name === properties.open} />
                </span>
            ))}
        </nav>
    );
}

/** Draw a page's header: its title, and its actions at the end, the last one as the primary action. */
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
