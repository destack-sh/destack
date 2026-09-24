import { color } from "@destack/theme/tokens.stylex";
import { type Accessor, Show } from "@destack/view";
import * as stylex from "@destack/style";

import { type Post, type PostContent } from "../generated/posts";
import { tokens } from "../style/tokens.stylex";
import { Breadcrumbs } from "./breadcrumbs";
import { publicationStyles } from "./publication.stylex";
import { Reader } from "./reader";
import { type ContentsEntry, ContentsTree, trackActiveHeading } from "./contents";
import { PageHeader } from "./header";
import { formatDate } from "../content/presentation";

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
    const activeHeading = trackActiveHeading(properties.post.tableOfContents);

    return (
        <Reader
            location={() => <Breadcrumbs items={[{ href: "/blog/", label: "Blog" }]} />}
            navigation={() => (
                <BlogNavigation
                    contents={properties.post.tableOfContents}
                    activeHeading={activeHeading}
                />
            )}
            pagination={() => <PostNavigation post={properties.post} posts={properties.posts} />}
            publication="journal"
            source={properties.post}
            tokenCount={properties.post.tokens}
        >
            <BlogArticleHeader post={properties.post} />
            <div class="markdown" innerHTML={properties.content.html} />
        </Reader>
    );
}

/** Render the article outline as the reading navigation. */
function BlogNavigation(properties: {
    contents: readonly ContentsEntry[];
    activeHeading: Accessor<string | undefined>;
}) {
    return (
        <nav aria-label="Article contents">
            <div {...stylex.attrs(publicationStyles.context)}>
                <a href="/blog/" {...stylex.attrs(publicationStyles.contextTitle)}>
                    Blog
                </a>
            </div>
            <div {...stylex.attrs(publicationStyles.collectionList)}>
                <ContentsTree entries={properties.contents} activeId={properties.activeHeading} />
            </div>
        </nav>
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
            <div {...stylex.attrs(styles.metadata)}>
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
            <nav aria-label="Post pagination" {...stylex.attrs(publicationStyles.pagination)}>
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
        <a {...stylex.attrs(publicationStyles.paginationLink)} href={properties.post.route}>
            {isNewer ? `← ${properties.post.title}` : `${properties.post.title} →`}
        </a>
    );
}

/** Journal article styles. */
const styles = stylex.create({
    metadata: {
        color: color.mutedForeground,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        gap: "0.5rem 1.25rem",
        paddingTop: "0.375rem",
    },
});
