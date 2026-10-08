import { text } from "@destack/theme/text";
import { color, font } from "@destack/theme/tokens.stylex";
import { Content } from "@destack/ui/content";
import { Prose } from "@destack/ui/prose";
import { type Heading, TableOfContents } from "@destack/ui/table-of-contents";
import { Show } from "@destack/view";
import * as style from "@destack/style";

import { type Post, type PostContent } from "../content/generated/posts";
import { Breadcrumbs } from "./breadcrumbs";
import { publicationStyles } from "./publication.stylex";
import { blocks } from "./blocks";
import { Reader } from "./reader";
import { PageHeader } from "./header";
import { formatDate } from "../content/presentation";

/** The distance from the viewport's top above which a heading counts as read: the 56px top bar and some air. */
const HEADER_OFFSET = 96;

/** Properties for one rendered blog article. */
type BlogArticleProperties = {
    /** The rendered post body. */
    content: PostContent;

    /** The current post. */
    post: Post;

    /** Every post in reverse chronological order. */
    posts: readonly Post[];
};

/** Render a blog post and its navigation. */
export function BlogArticle(properties: BlogArticleProperties) {
    return (
        <Reader
            location={() => <Breadcrumbs items={[{ href: "/blog/", label: "Blog" }]} />}
            navigation={() => <BlogNavigation contents={properties.post.tableOfContents} />}
            pagination={() => <PostNavigation post={properties.post} posts={properties.posts} />}
            publication="journal"
            source={properties.post}
            tokenCount={properties.post.tokens}
        >
            <BlogArticleHeader post={properties.post} />
            <Prose xstyle={[publicationStyles.reading, publicationStyles.prose]}>
                <Content tree={properties.content.tree} components={blocks} />
            </Prose>
        </Reader>
    );
}

/** Render the article outline as the reading navigation. */
function BlogNavigation(properties: { contents: readonly Heading[] }) {
    return (
        <div>
            <div {...style.attrs(publicationStyles.context)}>
                <a href="/blog/" {...style.attrs(publicationStyles.contextTitle)}>
                    Blog
                </a>
            </div>
            <div {...style.attrs(publicationStyles.collectionList)}>
                <TableOfContents
                    headings={properties.contents}
                    offset={HEADER_OFFSET}
                    xstyle={[text.subheadline, publicationStyles.outline]}
                />
            </div>
        </div>
    );
}

/** Properties for the blog article heading. */
type BlogArticleHeaderProperties = {
    /** The current post. */
    post: Post;
};

/** Render the post title and subtitle. */
function BlogArticleHeader(properties: BlogArticleHeaderProperties) {
    return (
        <PageHeader
            title={properties.post.title}
            variant="article"
            description={properties.post.subtitle}
        >
            <div {...style.attrs(text.caption, styles.metadata)}>
                <span>{properties.post.author}</span>
                <time datetime={properties.post.date}>{formatDate(properties.post.date)}</time>
            </div>
        </PageHeader>
    );
}

/** Properties for the adjacent post navigation. */
type PostNavigationProperties = {
    /** The current post. */
    post: Post;

    /** Every post in reverse chronological order. */
    posts: readonly Post[];
};

/** Render adjacent posts when they exist. */
function PostNavigation(properties: PostNavigationProperties) {
    // resolve neighbors from the canonical post order
    const index = () => properties.posts.findIndex((post) => post.slug === properties.post.slug);
    const newer = () => properties.posts[index() - 1];
    const older = () => properties.posts[index() + 1];

    return (
        <Show when={newer() || older()}>
            <nav
                aria-label="Post pagination"
                {...style.attrs(text.subheadline, publicationStyles.pagination)}
            >
                <Show when={newer()}>
                    {(post) => <PostNavigationLink direction="newer" post={post()} />}
                </Show>

                <Show when={older()}>
                    {(post) => <PostNavigationLink direction="older" post={post()} />}
                </Show>
            </nav>
        </Show>
    );
}

/** Properties for one adjacent post link. */
type PostNavigationLinkProperties = {
    /** The adjacent post direction. */
    direction: "newer" | "older";

    /** The adjacent post. */
    post: Post;
};

/** Render one adjacent post link. */
function PostNavigationLink(properties: PostNavigationLinkProperties) {
    const isNewer = properties.direction === "newer";

    return (
        <a {...style.attrs(publicationStyles.paginationLink)} href={properties.post.route}>
            {isNewer ? `← ${properties.post.title}` : `${properties.post.title} →`}
        </a>
    );
}

/** Journal article styles. */
const styles = style.create({
    metadata: {
        color: color.mutedForeground,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: font.code,
        rowGap: "0.5rem",
        columnGap: "1.25rem",
        paddingTop: "0.375rem",
    },
});
