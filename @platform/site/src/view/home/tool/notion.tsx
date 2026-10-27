import * as style from "@destack/style";

import { launch, press, type PressState } from "../verb/corner";
import { Glyph, type GlyphName } from "../figure/glyph";
import type { JSX } from "@destack/view";
import { site } from "../figure/site.stylex";
import { palette } from "../../palette.stylex";
import { notion } from "./notion.stylex";

/** A property's value: plain text, a status tag, or a person. */
export type NotionValue =
    | { kind: "text"; text: string }
    | { kind: "status"; text: string; isDraft: boolean }
    | { kind: "person"; name: string };

/** A property of a page: its glyph, its name, and its value. */
export type NotionProperty = readonly [glyph: GlyphName, key: string, value: NotionValue];

/** The top bar's tools: a public page's, or the editor's with its publish button. */
export type NotionTools = "public" | "editor";

/** Draw the Destack launch plan as a published page behind a public link. */
export function Notion() {
    return (
        <NotionPage
            crumbs={[["🚀", "Destack launch"]]}
            tools="public"
            icon="🚀"
            title="Destack launch"
            properties={[
                ["status", "Status", { kind: "status", text: "In progress", isDraft: false }],
                ["calendar", "Launch", { kind: "text", text: "October 24, 2025" }],
                ["calendar", "Demo", { kind: "text", text: "October 22, 2025 11:00 AM · Ines" }],
                ["person", "Owner", { kind: "person", name: "Florian" }],
            ]}
        >
            <p {...style.attrs(styles.notionCallout)}>
                <span aria-hidden="true" {...style.attrs(styles.notionEmoji)}>
                    💡
                </span>
                Ship to the waitlist on Thursday, then open signups on Friday.
            </p>
            <b {...style.attrs(styles.notionHeading)}>Before launch</b>
            <ul {...style.attrs(styles.notionTodos)}>
                {launch.map(([item, isDone]) => (
                    <li {...style.attrs(styles.notionTodo)}>
                        <span {...style.attrs(styles.notionBox, isDone && styles.notionBoxDone)}>
                            {isDone ? <Glyph name="check" size={12} weight={3} /> : undefined}
                        </span>
                        <span {...style.attrs(isDone && styles.notionDone)}>{item}</span>
                    </li>
                ))}
            </ul>
        </NotionPage>
    );
}

/** Draw a page: crumbs and tools, then cover, icon, title and properties, then the body. */
export function NotionPage(properties: {
    crumbs: readonly (readonly [icon: string, name: string])[];
    tools: NotionTools;
    icon: string;
    title: string;
    properties: readonly NotionProperty[];
    children: JSX.Element;
}) {
    return (
        <div data-component="NotionPage" {...style.attrs(styles.notion)}>
            <nav {...style.attrs(styles.notionBar)}>
                <span {...style.attrs(styles.notionCrumb)}>
                    {properties.crumbs.map(([icon, name], index) => (
                        <span {...style.attrs(styles.contents)}>
                            {index > 0 ? <span {...style.attrs(styles.muted)}>/</span> : undefined}
                            <span aria-hidden="true">{icon}</span>
                            {name}
                        </span>
                    ))}
                </span>
                {properties.tools === "public" ? (
                    <span {...style.attrs(styles.notionTools)}>
                        <Glyph name="search" size={18} weight={1.75} />
                        <Glyph name="share" size={18} weight={1.75} />
                        <Glyph name="more" size={18} weight={2.5} />
                        <span {...style.attrs(styles.notionButton)}>Get Notion free</span>
                    </span>
                ) : (
                    <span {...style.attrs(styles.notionTools)}>
                        <span>Share</span>
                        <span {...style.attrs(styles.notionPublish)}>Publish</span>
                    </span>
                )}
            </nav>
            <span {...style.attrs(styles.notionCover)} />
            <div {...style.attrs(styles.notionBody)}>
                <span aria-hidden="true" {...style.attrs(styles.notionIcon)}>
                    {properties.icon}
                </span>
                <b data-morph-title {...style.attrs(styles.notionTitle)}>
                    {properties.title}
                </b>
                <dl {...style.attrs(styles.notionProperties)}>
                    {properties.properties.map(([glyph, key, value]) => (
                        <div {...style.attrs(styles.contents)}>
                            <dt {...style.attrs(styles.notionKey)}>
                                <Glyph name={glyph} size={16} weight={1.75} />
                                {key}
                            </dt>
                            <dd {...style.attrs(styles.notionValue)}>
                                {value.kind === "status" ? (
                                    <span
                                        {...style.attrs(
                                            styles.notionStatus,
                                            value.isDraft && styles.notionDraft,
                                        )}
                                    >
                                        <span
                                            {...style.attrs(
                                                styles.notionDot,
                                                value.isDraft && styles.notionDraftDot,
                                            )}
                                        />
                                        {value.text}
                                    </span>
                                ) : value.kind === "person" ? (
                                    <>
                                        <span {...style.attrs(styles.notionPerson)}>
                                            {value.name.slice(0, 1)}
                                        </span>
                                        {value.name}
                                    </>
                                ) : (
                                    value.text
                                )}
                            </dd>
                        </div>
                    ))}
                </dl>
                <hr {...style.attrs(styles.notionRule)} />
                {properties.children}
            </div>
        </div>
    );
}

/** The strokes of the database's own glyphs, on a 24-unit grid. */
const marks = {
    table: ["M4 5h16v14H4z", "M4 10h16", "M10 10v9"],
    board: ["M4 5h16v14H4z", "M10 5v14", "M15 5v14"],
    text: ["M5 7h14", "M5 12h14", "M5 17h9"],
    select: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "m8.5 10.5 3.5 3.5 3.5-3.5"],
    sort: ["M8 4v16", "m4 8 4-4 4 4", "M16 20V4", "m12 16 4 4 4-4"],
    bolt: ["M13 3 5 14h6l-1 7 8-11h-6z"],
} satisfies Record<string, readonly string[]>;

/** The name each status tag shows. */
const statusNames: Readonly<Record<PressState, string>> = {
    pitched: "Pitched",
    replied: "Replied",
    scheduled: "Call booked",
};

/** Draw the press list as a database on a page of the launch workspace. */
export function NotionDatabase() {
    return (
        <div data-component="NotionDatabasePage" {...style.attrs(styles.database)}>
            {/* top bar */}
            <nav {...style.attrs(styles.notionBar)}>
                <span {...style.attrs(styles.notionCrumb)}>
                    <span aria-hidden="true">🚀</span>
                    Destack launch
                    <span {...style.attrs(styles.muted)}>/</span>
                    <span aria-hidden="true">📰</span>
                    Press
                </span>
                <span {...style.attrs(styles.notionTools)}>
                    <span>Share</span>
                    <Glyph name="clock" size={18} weight={1.75} />
                    <Glyph name="star" size={18} weight={1.75} />
                    <Glyph name="more" size={18} weight={2.5} />
                </span>
            </nav>

            {/* title */}
            <div {...style.attrs(styles.databaseBody)}>
                <b {...style.attrs(styles.databaseTitle)}>
                    <span aria-hidden="true">📰</span>
                    Press
                </b>

                {/* views and tools */}
                <span {...style.attrs(styles.views)}>
                    <span {...style.attrs(styles.view, styles.viewOn)}>
                        <Mark name="table" />
                        All press
                    </span>
                    <span {...style.attrs(styles.view, styles.viewBoard)}>
                        <Mark name="board" />
                        By status
                    </span>
                    <span {...style.attrs(styles.viewTools)}>
                        <Glyph name="filter" size={16} weight={1.75} />
                        <Mark name="sort" />
                        <Mark name="bolt" />
                        <Glyph name="search" size={16} weight={1.75} />
                        <span {...style.attrs(styles.newButton)}>
                            New
                            <Glyph name="caret" size={12} weight={2.5} />
                        </span>
                    </span>
                </span>

                {/* table */}
                <div {...style.attrs(styles.table)}>
                    <span {...style.attrs(styles.row, styles.headRow)}>
                        <span {...style.attrs(styles.cell)}>
                            <span {...style.attrs(styles.typeMark)}>Aa</span>
                            Name
                        </span>
                        <span {...style.attrs(styles.cell)}>
                            <Mark name="text" />
                            Outlet
                        </span>
                        <span {...style.attrs(styles.cell)}>
                            <Mark name="select" />
                            Status
                        </span>
                        <span {...style.attrs(styles.cell, styles.wide)}>
                            <Glyph name="calendar" size={15} weight={1.75} />
                            Last contact
                        </span>
                    </span>
                    {press.map(([name, outlet, state, last]) => (
                        <span {...style.attrs(styles.row)}>
                            <span {...style.attrs(styles.cell, styles.title)}>{name}</span>
                            <span {...style.attrs(styles.cell)}>{outlet}</span>
                            <span {...style.attrs(styles.cell)}>
                                <span
                                    {...style.attrs(
                                        styles.tag,
                                        state === "scheduled"
                                            ? styles.tagGreen
                                            : state === "replied"
                                              ? styles.tagBlue
                                              : styles.tagYellow,
                                    )}
                                >
                                    {statusNames[state]}
                                </span>
                            </span>
                            <span {...style.attrs(styles.cell, styles.wide)}>{last}, 2025</span>
                        </span>
                    ))}
                    <span {...style.attrs(styles.newRow)}>
                        <Glyph name="plus" size={15} weight={1.75} />
                        New page
                    </span>
                </div>
            </div>
        </div>
    );
}

/** Draw one of the database's own glyphs in the current colour. */
function Mark(properties: { name: keyof typeof marks }) {
    return (
        <svg aria-hidden="true" viewBox="0 0 24 24" {...style.attrs(styles.mark)}>
            {marks[properties.name].map((path) => (
                <path d={path} />
            ))}
        </svg>
    );
}

/** The notion page styles. */
const styles = style.create({
    notion: {
        backgroundColor: notion.surface,
        color: notion.ink,
        fontFamily: notion.face,
        height: "100%",
        overflow: "hidden",
    },
    notionBar: {
        fontFamily: site.system,
        alignItems: "center",
        display: "flex",
        height: "45px",
        justifyContent: "space-between",
        paddingInline: "12px",
    },
    notionCrumb: {
        alignItems: "center",
        display: "flex",
        fontSize: "14px",
        gap: "6px",
        paddingInline: "6px",
    },
    notionTools: {
        alignItems: "center",
        color: notion.ink,
        display: "flex",
        gap: "16px",
    },
    notionButton: {
        backgroundColor: notion.charcoal,
        borderRadius: "6px",
        color: notion.surface,
        fontSize: "14px",
        fontWeight: 500,
        paddingBlock: "6px",
        paddingInline: "10px",
    },
    notionCover: {
        backgroundImage: `linear-gradient(120deg, ${notion.paper} 0%, ${notion.tan} 50%, ${notion.clay} 100%)`,
        display: "block",
        height: { default: "96px", [site.narrow]: "64px" },
    },
    notionBody: {
        display: "grid",
        paddingInline: { default: "72px", [site.narrow]: "20px" },
    },
    notionIcon: {
        fontSize: { default: "78px", [site.narrow]: "56px" },
        lineHeight: 1,
        marginTop: "-40px",
    },
    notionTitle: {
        fontSize: { default: "40px", [site.narrow]: "30px" },
        fontWeight: 700,
        lineHeight: 1.2,
        marginBottom: "12px",
        marginTop: "16px",
    },
    notionProperties: {
        alignItems: "center",
        display: "grid",
        fontSize: "14px",
        gridAutoRows: "34px",
        gridTemplateColumns: {
            default: "160px minmax(0, 1fr)",
            [site.narrow]: "104px minmax(0, 1fr)",
        },
        margin: 0,
    },
    notionKey: {
        alignItems: "center",
        color: `color-mix(in srgb, ${notion.ink} 65%, transparent)`,
        display: "flex",
        gap: "8px",
    },
    notionValue: {
        alignItems: "center",
        display: "flex",
        margin: 0,
    },
    notionStatus: {
        alignItems: "center",
        backgroundColor: notion.sky,
        borderRadius: "10px",
        color: notion.navy,
        display: "inline-flex",
        gap: "6px",
        paddingBlock: "1px",
        paddingInlineStart: "8px",
        paddingInlineEnd: "9px",
    },
    contents: {
        display: "contents",
    },
    notionPublish: {
        backgroundColor: notion.link,
        borderRadius: "6px",
        color: notion.onLink,
        fontWeight: 500,
        paddingBlock: "3px",
        paddingInline: "10px",
    },
    notionDraft: {
        backgroundColor: notion.line,
        color: notion.ink,
    },
    notionDraftDot: {
        backgroundColor: notion.muted,
    },
    notionDot: {
        backgroundColor: notion.blue,
        borderRadius: "50%",
        height: "8px",
        width: "8px",
    },
    notionPerson: {
        alignItems: "center",
        backgroundColor: palette.teal,
        borderRadius: "50%",
        color: "white",
        display: "flex",
        fontSize: "10px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        marginRight: "6px",
        width: "20px",
    },
    notionRule: {
        borderColor: `color-mix(in srgb, ${notion.ink} 9%, transparent)`,
        borderStyle: "solid",
        borderBlockStartWidth: "1px",
        borderBlockEndWidth: "0",
        borderInlineWidth: "0",
        marginBlockStart: "16px",
        marginBlockEnd: "20px",
        marginInline: "0",
        width: "100%",
    },
    notionCallout: {
        alignItems: "flex-start",
        backgroundColor: notion.sand,
        borderRadius: "4px",
        display: "flex",
        fontSize: "16px",
        gap: "8px",
        lineHeight: 1.5,
        margin: 0,
        paddingBlock: "16px",
        paddingInlineStart: "12px",
        paddingInlineEnd: "16px",
    },
    notionEmoji: {
        fontSize: "21px",
        lineHeight: 1.1,
    },
    notionHeading: {
        fontSize: "24px",
        fontWeight: 600,
        marginBottom: "8px",
        marginTop: "32px",
    },
    notionTodos: {
        display: "grid",
        fontSize: "16px",
        gap: "6px",
        lineHeight: 1.5,
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    notionTodo: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
    },
    notionBox: {
        alignItems: "center",
        borderColor: notion.ink,
        borderRadius: "3px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        color: "white",
        display: "flex",
        flexShrink: 0,
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },
    notionBoxDone: {
        backgroundColor: notion.link,
        borderColor: notion.link,
    },
    notionDone: {
        opacity: 0.375,
        textDecorationLine: "line-through",
    },

    // database
    database: {
        backgroundColor: notion.surface,
        color: notion.ink,
        fontFamily: notion.sans,
        fontSize: "14px",
        height: "100%",
        overflow: "hidden",
    },
    muted: {
        color: notion.muted,
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        height: "16px",
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.75,
        width: "16px",
    },
    databaseBody: {
        display: "grid",
        paddingInline: { default: "96px", [site.compact]: "48px", [site.narrow]: "16px" },
    },
    databaseTitle: {
        alignItems: "center",
        display: "flex",
        fontSize: { default: "40px", [site.narrow]: "30px" },
        fontWeight: 700,
        gap: "14px",
        lineHeight: 1.2,
        marginBlockEnd: "20px",
        marginBlockStart: { default: "56px", [site.narrow]: "28px" },
    },
    views: {
        alignItems: "center",
        borderBottomColor: notion.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: notion.muted,
        display: "flex",
        gap: "4px",
    },
    view: {
        alignItems: "center",
        borderBottomColor: "transparent",
        borderBottomStyle: "solid",
        borderBottomWidth: "2px",
        display: "flex",
        fontWeight: 500,
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "4px",
        whiteSpace: "nowrap",
    },
    viewOn: {
        borderBottomColor: notion.ink,
        color: notion.ink,
    },
    viewBoard: {
        display: { default: "flex", [site.narrow]: "none" },
    },
    viewTools: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
        marginInlineStart: "auto",
    },
    newButton: {
        alignItems: "center",
        backgroundColor: notion.link,
        borderRadius: "6px",
        color: "white",
        display: "flex",
        fontWeight: 500,
        gap: "8px",
        paddingBlock: "4px",
        paddingInline: "10px",
    },
    table: {
        display: "grid",
    },
    row: {
        borderBottomColor: notion.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "grid",
        gridTemplateColumns: {
            default: "minmax(0, 1.3fr) minmax(0, 1.3fr) minmax(0, 1fr) minmax(0, 0.9fr)",
            [site.narrow]: "minmax(0, 1.2fr) minmax(0, 1fr) minmax(0, 1fr)",
        },
        minHeight: "34px",
    },
    headRow: {
        color: notion.muted,
    },
    cell: {
        alignItems: "center",
        borderInlineEndColor: notion.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        display: "flex",
        gap: "6px",
        minWidth: 0,
        overflow: "hidden",
        paddingInline: "8px",
        whiteSpace: "nowrap",
    },
    wide: {
        borderInlineEndWidth: "0",
        display: { default: "flex", [site.narrow]: "none" },
    },
    typeMark: {
        fontSize: "13px",
    },
    title: {
        fontWeight: 500,
        textDecorationColor: notion.line,
        textDecorationLine: "underline",
        textDecorationThickness: "1px",
        textUnderlineOffset: "3px",
    },
    tag: {
        borderRadius: "3px",
        overflow: "hidden",
        paddingInline: "6px",
        textOverflow: "ellipsis",
    },
    tagGreen: {
        backgroundColor: notion.green,
        color: notion.greenInk,
    },
    tagBlue: {
        backgroundColor: notion.sky,
        color: notion.navy,
    },
    tagYellow: {
        backgroundColor: notion.yellow,
        color: notion.yellowInk,
    },
    newRow: {
        alignItems: "center",
        borderBottomColor: notion.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: notion.muted,
        display: "flex",
        gap: "6px",
        minHeight: "34px",
        paddingInline: "8px",
    },
});
