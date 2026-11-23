import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { Avatar, AvatarFallback } from "@destack/ui/avatar";
import { Badge } from "@destack/ui/badge";
import { Button } from "@destack/ui/button";
import { Separator } from "@destack/ui/separator";
import { color, radius, stroke, swatch } from "@destack/theme/tokens.stylex";
import { createEffect, type JSX, useEnvironment } from "@destack/view";
import { requestFrame } from "@destack/view/primitives/raf";

import { AgentAvatar, isAgent } from "./agent";
import { paperGlyphOf } from "./planet";

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
        backgroundColor: color.background,
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
        overflowX: { default: "hidden", [NARROW]: "auto" },
        overflowY: { default: "auto", [NARROW]: "hidden" },
        paddingBlock: { default: "0.75rem", [NARROW]: "0.375rem" },
        position: "relative",
        scrollbarWidth: "none",
    },
    railWide: {
        display: { default: "flex", [NARROW]: "none" },
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
        backgroundColor: swatch.accentSolid,
    },
    railInbox: {
        position: "relative",
    },
    railBadge: {
        borderRadius: radius.full,
        fontSize: "0.625rem",
        fontVariantNumeric: "tabular-nums",
        height: "0.875rem",
        insetBlockStart: "-0.125rem",
        insetInlineEnd: "-0.125rem",
        minWidth: "0.875rem",
        paddingInline: "0.1875rem",
        position: "absolute",
    },
    railRule: {
        display: { default: "block", [NARROW]: "none" },
        flexShrink: 0,
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
        // fade on the page's own layer, so the mask is never redrawn as the site scrolls
        willChange: "transform",
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
    note: {
        color: color.mutedForeground,
        display: "block",
        lineHeight: "1.25rem",
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
    onOpen?: (name: string) => void;
}) {
    // scroll the open app into the rail's view on the next frame, reading the layout that frame lays out anyway
    const environment = useEnvironment();
    let rail: HTMLElement | undefined;
    createEffect(
        () => properties.open,
        (open) =>
            requestFrame(() => {
                const link = rail?.querySelector<HTMLElement>(`[data-app="${open}"]`);
                if (rail !== undefined && link !== null && link !== undefined) {
                    rail.scrollTop = link.offsetTop - rail.clientHeight / 2;
                }
            }, environment.display),
    );

    return (
        <nav ref={rail} data-component="AppRail" data-flip-enter {...style.attrs(appStyles.rail)}>
            <Avatar size="sm" color="cyan" title="Your space" xstyle={appStyles.railWide}>
                <AvatarFallback>F</AvatarFallback>
            </Avatar>
            <Button variant="ghost" size="icon-sm" aria-label="Search" xstyle={appStyles.railWide}>
                <AppGlyph name="search" />
            </Button>
            <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Inbox"
                xstyle={[appStyles.railWide, appStyles.railInbox]}
            >
                <AppGlyph name="notify" />
                <Badge xstyle={appStyles.railBadge}>3</Badge>
            </Button>
            <Separator xstyle={appStyles.railRule} />
            {properties.apps.map(([name, label]) => (
                <Button
                    variant="ghost"
                    size="icon-sm"
                    data-component="AppLink"
                    data-app={name}
                    data-flip={`icon-${name}`}
                    title={label}
                    aria-label={label}
                    aria-current={name === properties.open ? "page" : undefined}
                    onClick={() => properties.onOpen?.(name)}
                    xstyle={[name === properties.open && state.selected]}
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
                </Button>
            ))}
        </nav>
    );
}

/** Draw a page's header. */
export function AppHeader(properties: { title: JSX.Element; children?: JSX.Element }) {
    return (
        <div data-component="PageHeader" {...style.attrs(appStyles.toolbar)}>
            <b data-component="PageTitle" data-flip="title" {...style.attrs(appText.title)}>
                {properties.title}
            </b>
            <div {...style.attrs(appStyles.tools)}>{properties.children}</div>
        </div>
    );
}
