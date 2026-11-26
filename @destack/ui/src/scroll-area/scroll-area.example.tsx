import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, size, space, stroke, width } from "@destack/theme/tokens.stylex";
import { For } from "@destack/view";
import { Badge } from "../badge/index.ts";
import { ScrollArea } from "./scroll-area.tsx";

/** The height of the tag list. */
const styles = style.create({
    tags: {
        width: width.hoverCard,
        height: `calc(5 * ${size[3]})`,
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
    },
    strip: {
        width: width.hoverCard,
        paddingBlock: space[3],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
    },
    labels: {
        display: "flex",
        gap: space[2],
        width: "max-content",
        paddingInline: space[3],
    },
    tag: {
        margin: 0,
        paddingBlock: space[2],
        paddingInline: space[3],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
});

/** The tags of a notebook, more than the region shows at once. */
const TAGS = Array.from({ length: 40 }, (_, index) => `tag-${index + 1}`);

/** A long list of tags in a scrolling region. */
export const scrollAreaNotebookTags = defineExample({
    of: ScrollArea,
    name: "notebookTags",
    description: t`A long list of tags in a scrolling region.`,
    properties: {
        orientation: "vertical",
        "aria-label": "Tags",
    },
    render: (properties) => (
        <ScrollArea {...properties} xstyle={styles.tags}>
            <For each={TAGS}>{(tag) => <p {...style.attrs(text.sm, styles.tag)}>{tag}</p>}</For>
        </ScrollArea>
    ),
});

/** A row of labels wider than its region, scrolling sideways. */
export const scrollAreaLabelStrip = defineExample({
    of: ScrollArea,
    name: "labelStrip",
    description: t`A row of labels wider than its region, scrolling sideways.`,
    properties: {
        orientation: "horizontal",
        "aria-label": "Labels",
    },
    render: (properties) => (
        <ScrollArea {...properties} xstyle={styles.strip}>
            <div {...style.attrs(styles.labels)}>
                <For each={TAGS.slice(0, 12)}>
                    {(tag) => <Badge variant="secondary">{tag}</Badge>}
                </For>
            </div>
        </ScrollArea>
    ),
});
