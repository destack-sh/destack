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

/// Properties for one rendered blog article.
type BlogArticleProps = {
    /// The rendered post body.
    content: PostContent;

    /// The current post.
    post: Post;

    /// Every post in reverse chronological order.
    posts: readonly Post[];
};

/// Render a blog post and its navigation.
export function BlogArticle(props: BlogArticleProps) {
    const activeHeading = trackActiveHeading(props.post.tableOfContents);

    return (
        <Reader
            location={() => <Breadcrumbs items={[{ href: "/blog/", label: "Blog" }]} />}
            navigation={() => (
                <BlogNavigation
                    contents={props.post.tableOfContents}
                    activeHeading={activeHeading}
                />
            )}
            pagination={() => <PostNavigation post={props.post} posts={props.posts} />}
            publication="journal"
            source={props.post}
            tokenCount={props.post.tokens}
        >
            <BlogArticleHeader post={props.post} />
            <div class="markdown" innerHTML={props.content.html} />
        </Reader>
    );
}

/// Render the article outline as the reading navigation.
function BlogNavigation(props: {
    contents: readonly ContentsEntry[];
    activeHeading: Accessor<string>;
}) {
    return (
        <nav aria-label="Article contents">
            <div {...stylex.attrs(publicationStyles.context)}>
                <a href="/blog/" {...stylex.attrs(publicationStyles.contextTitle)}>
                    Blog
                </a>
            </div>
            <div {...stylex.attrs(publicationStyles.collectionList)}>
                <ContentsTree entries={props.contents} activeId={props.activeHeading} />
            </div>
        </nav>
    );
}

/// Properties for the blog article heading.
type BlogArticleHeaderProps = {
    /// The current post.
    post: Post;
};

/// Render the post title and subtitle.
function BlogArticleHeader(props: BlogArticleHeaderProps) {
    return (
        <PageHeader title={props.post.title} variant="article" description={props.post.subtitle}>
            <div {...stylex.attrs(styles.metadata)}>
                <span>{props.post.author}</span>
                <time datetime={props.post.date}>{formatDate(props.post.date)}</time>
            </div>
        </PageHeader>
    );
}

/// Properties for the adjacent post navigation.
type PostNavigationProps = {
    /// The current post.
    post: Post;

    /// Every post in reverse chronological order.
    posts: readonly Post[];
};

/// Render adjacent posts when they exist.
function PostNavigation(props: PostNavigationProps) {
    // resolve neighbors from the canonical post order
    const index = () => props.posts.findIndex((post) => post.slug === props.post.slug);
    const newer = () => props.posts[index() - 1];
    const older = () => props.posts[index() + 1];

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

/// Properties for one adjacent post link.
type PostNavigationLinkProps = {
    /// The adjacent post direction.
    direction: "newer" | "older";

    /// The adjacent post.
    post: Post;
};

/// Render one adjacent post link.
function PostNavigationLink(props: PostNavigationLinkProps) {
    const isNewer = props.direction === "newer";

    return (
        <a {...stylex.attrs(publicationStyles.paginationLink)} href={props.post.route}>
            {isNewer ? `← ${props.post.title}` : `${props.post.title} →`}
        </a>
    );
}

/// Journal article styles.
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
