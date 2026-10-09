import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createMemo,
    createSignal,
    For,
    type JSX,
    omit,
    onSettled,
    Show,
} from "@destack/view";
import { createEventListener } from "@destack/view/primitives/event-listener";

/** The styles of a table of contents and its elements. */
const styles = style.create({
    list: {
        display: "flex",
        flexDirection: "column",
    },
    nested: {
        paddingInlineStart: space[3],
    },
    navigation: {
        color: color.mutedForeground,
    },
    link: {
        display: "block",
        paddingBlock: space[1],
        color: {
            default: "inherit",
            ":hover": { default: null, [media.hover]: color.primary },
            ":is([aria-current=location])": color.foreground,
        },
        fontWeight: { default: null, ":is([aria-current=location])": weight.semibold },
        transitionProperty: "color",
        transitionDuration: motion.durationShort,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
});

/** A heading of the page a table of contents lists. */
export interface Heading {
    /** The heading's level, 1 for `h1` to 6 for `h6`. */
    readonly depth: number;
    /** The heading element's id, which its link jumps to. */
    readonly id: string;
    /** The heading's text. */
    readonly text: string;
}

/** A heading with the headings nested below it. */
interface Section extends Heading {
    /** The headings one level deeper until the next heading at this level or above. */
    readonly children: Section[];
}

/** The properties of a table of contents, the native navigation element's attributes included. */
export interface TableOfContentsProperties extends Omit<JSX.HTMLAttributes<HTMLElement>, "class"> {
    /** The page's headings in document order. */
    readonly headings: readonly Heading[];
    /** The distance in pixels from the viewport's top above which a heading counts as read, such as a sticky header's height. */
    readonly offset?: number;
    /** The StyleX styles applied after the navigation's styles. */
    readonly xstyle?: style.Styles;
}

/** Render the navigation landmark of a page's headings, marking the section being read; its links take the navigation's color. */
export function TableOfContents(properties: TableOfContentsProperties): JSX.Element {
    // nest the headings and follow the one being read
    const rest = omit(properties, "headings", "offset", "xstyle", "style");
    const sections = createMemo(() => outline(properties.headings));
    const current = createCurrentHeading(
        () => properties.headings.map((heading) => heading.id),
        () => properties.offset ?? 0,
    );

    return (
        <nav
            aria-label={"On this page"}
            data-slot="table-of-contents"
            {...rest}
            {...style.attributes(
                [text.callout, styles.navigation, properties.xstyle],
                properties.style,
            )}
        >
            <SectionList sections={sections()} current={current} isNested={false} />
        </nav>
    );
}

/** Render one level of the outline. */
function SectionList(properties: {
    readonly sections: readonly Section[];
    readonly current: Accessor<string | undefined>;
    readonly isNested: boolean;
}): JSX.Element {
    return (
        <ol {...style.attrs(styles.list, properties.isNested && styles.nested)}>
            <For each={properties.sections}>
                {(section) => (
                    <li>
                        {/* scroll natively within this browsing context, which routers leave unclaimed */}
                        <a
                            href={`#${section.id}`}
                            target="_self"
                            aria-current={
                                properties.current() === section.id ? "location" : undefined
                            }
                            {...style.attrs(styles.link)}
                        >
                            {section.text}
                        </a>
                        <Show when={section.children.length > 0}>
                            <SectionList
                                sections={section.children}
                                current={properties.current}
                                isNested
                            />
                        </Show>
                    </li>
                )}
            </For>
        </ol>
    );
}

/** Follow the heading being read: the last one scrolled past the offset, else the first. */
export function createCurrentHeading(
    ids: Accessor<readonly string[]>,
    offset: Accessor<number>,
): Accessor<string | undefined> {
    // start at the first heading
    const [current, setCurrent] = createSignal<string | undefined>(ids()[0], { ownedWrite: true });

    // read the heading being read once per frame while the page scrolls or resizes
    let frame = 0;
    const update = () => {
        cancelAnimationFrame(frame);
        frame = requestAnimationFrame(() => setCurrent(currentOf(ids(), offset())));
    };
    createEventListener(() => window, ["scroll", "resize"], update, { passive: true });
    onSettled(() => {
        update();

        return () => cancelAnimationFrame(frame);
    });

    return current;
}

/** Pick the last heading whose top has passed the offset, else the first heading. */
function currentOf(ids: readonly string[], offset: number): string | undefined {
    const passed = ids.filter((id) => {
        const element = document.getElementById(id);

        return element !== null && element.getBoundingClientRect().top <= offset + 1;
    });

    return passed.at(-1) ?? ids[0];
}

/** Nest each heading below the nearest preceding shallower heading. */
function outline(headings: readonly Heading[]): Section[] {
    // keep the top-level sections and the chain of open parents
    const roots: Section[] = [];
    const parents: Section[] = [];

    // close parents at the heading's depth or deeper, then attach it to the nearest one left
    for (const heading of headings) {
        let parent = parents.at(-1);
        while (parent !== undefined && parent.depth >= heading.depth) {
            parents.pop();
            parent = parents.at(-1);
        }
        const section: Section = { ...heading, children: [] };
        if (parent === undefined) {
            roots.push(section);
        } else {
            parent.children.push(section);
        }
        parents.push(section);
    }

    return roots;
}
