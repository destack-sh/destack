import { text } from "@destack/theme/text";
import { publicationStyles } from "../reader/publication.stylex";
import { type Heading, TableOfContents } from "../reader/table-of-contents";

/** Outline an article's headings, marking the one being read below an offset. */
export default function Outline(properties: {
    readonly headings: readonly Heading[];
    readonly offset: number;
}) {
    return (
        <TableOfContents
            headings={properties.headings}
            offset={properties.offset}
            xstyle={[text.subheadline, publicationStyles.outline]}
        />
    );
}
