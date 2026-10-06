import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { size } from "@destack/theme/tokens.stylex";
import { For } from "solid-js";
import { ScrollArea } from "./scroll-area.tsx";

/** The height of the tag list. */
const styles = style.create({
    tags: { height: `calc(5 * ${size[3]})` },
});

/** The tags of a notebook, more than the region shows at once. */
const TAGS = Array.from({ length: 40 }, (_, index) => `tag-${index + 1}`);

/** A long list of tags in a scrolling region. */
export const scrollAreaNotebookTags = defineExample({
    of: ScrollArea,
    name: "notebook-tags",
    description: "a long list of tags in a scrolling region",
    render: () => (
        <ScrollArea aria-label="Tags" style={styles.tags}>
            <For each={TAGS}>{(tag) => <p>{tag}</p>}</For>
        </ScrollArea>
    ),
});
