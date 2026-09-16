import { A } from "@solidjs/router";
import { Show, type Accessor } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { publicationStyles } from "./publication.stylex";

import { type Post, type PostContent } from "../generated/posts";
import { Breadcrumbs } from "./breadcrumbs";
import { tokens } from "../style/tokens.stylex";
import { Reader } from "./reader";
import { ContentsTree, trackActiveHeading, type ContentsEntry } from "./contents";
import { PageHeader } from "./header";
import { formatDate, formatReadTime } from "../content/presentation";

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
            location={() => (
                <Breadcrumbs items={[{ href: "/blog/", label: "Blog" }]} />
            )}
            navigation={() => <BlogNavigation contents={props.post.tableOfContents} activeHeading={activeHeading} />}
            metadata={() => <>
                <span>{props.post.author}</span>
                <time dateTime={props.post.date}>{formatDate(props.post.date)}</time>
                <span>{formatReadTime(props.post.tokens)}</span>
            </>}
            publication="journal"
            source={props.post}
        >
            <BlogArticleHeader post={props.post} />
            <div class="markdown" innerHTML={props.content.html} />
            <PostNavigation post={props.post} posts={props.posts} />
        </Reader>
    );
}

/// Blog articles use their heading outline as the primary reading navigation.
function BlogNavigation(props: { contents: readonly ContentsEntry[]; activeHeading: Accessor<string>; }) {
    return (
        <nav aria-label="Article contents" {...stylex.attrs(styles.book)}>
            <div class="collection-context"><A href="/blog/">← Blog</A></div>
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
        <PageHeader title={props.post.title} variant="article" description={props.post.subtitle} />
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
    const index = () =>
        props.posts.findIndex((post) => post.slug === props.post.slug);
    const newer = () => props.posts[index() - 1];
    const older = () => props.posts[index() + 1];

    return (
        <Show when={newer() || older()}>
            <nav
                aria-label="post navigation"
                {...stylex.attrs(styles.pagination)}
            >
                <Show when={newer()}>
                    {(post) => (
                        <PostNavigationLink direction="newer" post={post()} />
                    )}
                </Show>

                <Show when={older()}>
                    {(post) => (
                        <PostNavigationLink direction="older" post={post()} />
                    )}
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
        <A {...stylex.attrs(styles.paginationLink)} href={props.post.route}>
            {isNewer ? `← ${props.post.title}` : `${props.post.title} →`}
        </A>
    );
}

/// Journal navigation and article styles.
const styles = stylex.create({
    book: {
        alignContent: "start",
        display: "grid",
        gap: 0,
    },
    pagination: {
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        gap: "1rem 2rem",
        justifyContent: "space-between",
        marginTop: "2rem",
        paddingTop: "var(--content-section-gap)",
    },
    paginationLink: {
        color: tokens.ink,
        ":hover": {
            color: tokens.accent,
        },
    },
});
