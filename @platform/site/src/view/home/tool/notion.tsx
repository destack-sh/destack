import * as style from "@destack/style";

import { launch } from "../verb/corner";
import { Glyph } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { palette } from "../../palette.stylex";
import { notion } from "./notion.stylex";

/** Draw the Destack launch plan as a published page behind a public link. */
export function Notion() {
    return (
        <div data-component="NotionPage" {...style.attrs(styles.notion)}>
            <nav {...style.attrs(styles.notionBar)}>
                <span {...style.attrs(styles.notionCrumb)}>
                    <span aria-hidden="true">🚀</span>
                    Destack launch
                </span>
                <span {...style.attrs(styles.notionTools)}>
                    <Glyph name="search" size={18} weight={1.75} />
                    <Glyph name="share" size={18} weight={1.75} />
                    <Glyph name="more" size={18} weight={2.5} />
                    <span {...style.attrs(styles.notionButton)}>Get Notion free</span>
                </span>
            </nav>
            <span {...style.attrs(styles.notionCover)} />
            <div {...style.attrs(styles.notionBody)}>
                <span aria-hidden="true" {...style.attrs(styles.notionIcon)}>
                    🚀
                </span>
                <b data-morph-title {...style.attrs(styles.notionTitle)}>
                    Destack launch
                </b>
                <dl {...style.attrs(styles.notionProperties)}>
                    <dt {...style.attrs(styles.notionKey)}>
                        <Glyph name="status" size={16} weight={1.75} />
                        Status
                    </dt>
                    <dd {...style.attrs(styles.notionValue)}>
                        <span {...style.attrs(styles.notionStatus)}>
                            <span {...style.attrs(styles.notionDot)} />
                            In progress
                        </span>
                    </dd>
                    <dt {...style.attrs(styles.notionKey)}>
                        <Glyph name="calendar" size={16} weight={1.75} />
                        Launch
                    </dt>
                    <dd {...style.attrs(styles.notionValue)}>October 24, 2025</dd>
                    <dt {...style.attrs(styles.notionKey)}>
                        <Glyph name="calendar" size={16} weight={1.75} />
                        Demo
                    </dt>
                    <dd {...style.attrs(styles.notionValue)}>October 22, 2025 11:00 AM · Ines</dd>
                    <dt {...style.attrs(styles.notionKey)}>
                        <Glyph name="person" size={16} weight={1.75} />
                        Owner
                    </dt>
                    <dd {...style.attrs(styles.notionValue)}>
                        <span {...style.attrs(styles.notionPerson)}>F</span>
                        Florian
                    </dd>
                </dl>
                <hr {...style.attrs(styles.notionRule)} />
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
                            <span
                                {...style.attrs(styles.notionBox, isDone && styles.notionBoxDone)}
                            >
                                {isDone ? <Glyph name="check" size={12} weight={3} /> : undefined}
                            </span>
                            <span {...style.attrs(isDone && styles.notionDone)}>{item}</span>
                        </li>
                    ))}
                </ul>
            </div>
        </div>
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
});
