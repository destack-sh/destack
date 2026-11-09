import { Content } from "@destack/ui/content";
import { Prose } from "@destack/ui/prose";
import * as style from "@destack/style";
import Outline from "../islands/outline";

import type { Policy, RenderedContent } from "../content/collection";
import { Breadcrumbs } from "./breadcrumbs";
import { publicationStyles } from "./publication.stylex";
import { lattice } from "../layout/lattice.stylex";
import { blocks } from "./blocks";
import { Reader } from "./reader";
import { PageHeader } from "./header";

/** The distance from the viewport's top above which a heading counts as read. */
const HEADER_OFFSET = 96;

/** Render a policy with its outline. */
export function PolicyArticle(properties: {
    /** The rendered page body. */
    content: RenderedContent;
    /** The policy. */
    page: Policy;
}) {
    return (
        <Reader
            location={() => <Breadcrumbs items={[]} />}
            navigation={() => (
                <div>
                    <div {...style.attrs(lattice.ruleBottom, publicationStyles.context)}>
                        <span {...style.attrs(publicationStyles.contextTitle)}>Legal</span>
                    </div>
                    <div {...style.attrs(publicationStyles.collectionList)}>
                        <Outline
                            headings={properties.page.tableOfContents}
                            offset={HEADER_OFFSET}
                        />
                    </div>
                </div>
            )}
            publication="journal"
            source={properties.page}
            tokenCount={properties.page.tokens}
        >
            <PageHeader
                title={properties.page.title}
                variant="article"
                description={properties.page.description}
            />
            <Prose xstyle={[publicationStyles.reading, publicationStyles.prose]}>
                <Content tree={properties.content.tree} components={blocks} />
            </Prose>
        </Reader>
    );
}
