import { A } from "@solidjs/router";
import { type Accessor, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { type Post, type PostContent } from "../generated/posts";
import { ContentsTree, type ContentsEntry } from "./contents";
import { tokens } from "../style/tokens.stylex";
import { Reader } from "./reader";

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
    return (
        <Reader
            contents={props.post.tableOfContents}
            location={() => <BlogLocation post={props.post} />}
            navigation={(activeHeading) => (
                <BlogNavigation
                    activeHeading={activeHeading}
                    contents={props.post.tableOfContents}
                />
            )}
            publication="journal"
            source={props.post}
        >
            <BlogArticleHeader post={props.post} />
            <div class="markdown" innerHTML={props.content.html} />
            <PostNavigation post={props.post} posts={props.posts} />
        </Reader>
    );
}

/// Properties for the blog article location.
type BlogLocationProps = {
    /// The current post.
    post: Post;
};

/// Render the post path and publication metadata.
function BlogLocation(props: BlogLocationProps) {
    return (
        <div {...stylex.attrs(styles.location)}>
            <span>
                <time {...stylex.attrs(styles.locationDate)}>{props.post.date}</time>
                {" / "}{props.post.author}
            </span>
        </div>
    );
}

/// Properties for the blog article navigation.
type BlogNavigationProps = {
    /// The currently active heading identifier.
    activeHeading: Accessor<string>;

    /// The headings in the current article.
    contents: readonly ContentsEntry[];
};

/// Render the blog article navigation.
function BlogNavigation(props: BlogNavigationProps) {
    return (
        <nav aria-label="blog" {...stylex.attrs(styles.book)}>
            <A {...stylex.attrs(styles.bookTitle)} href="/blog/">
                blog
            </A>
            <ContentsTree activeId={props.activeHeading} entries={props.contents} />
        </nav>
    );
}

/// Properties for the blog article heading.
type BlogArticleHeaderProps = {
    /// The current post.
    post: Post;
};

/// Render the post title and metadata.
function BlogArticleHeader(props: BlogArticleHeaderProps) {
    return (
        <header {...stylex.attrs(styles.articleHeader)}>
            <h1 {...stylex.attrs(styles.articleTitle)}>{props.post.title}</h1>
            <p {...stylex.attrs(styles.articleSubtitle)}>{props.post.subtitle}</p>
        </header>
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
        <nav aria-label="post navigation" {...stylex.attrs(styles.pagination)}>
            <Show when={newer()}>
                {(post) => <PostNavigationLink direction="newer" post={post()} />}
            </Show>

            <Show when={older()}>
                {(post) => <PostNavigationLink direction="older" post={post()} />}
            </Show>
        </nav>
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
        <A href={props.post.route}>
            {isNewer ? `← ${props.post.title}` : `${props.post.title} →`}
        </A>
    );
}

/// Journal navigation and article styles.
const styles = stylex.create({
    articleHeader: {
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "grid",
        gap: `calc(${tokens.publicationSpace} * 2)`,
        paddingBottom: `calc(${tokens.publicationSpace} * 4)`,
        paddingTop: `calc(${tokens.publicationSpace} * 3)`,
    },
    articleSubtitle: {
        color: tokens.soft,
        fontFamily: tokens.textFont,
        fontSize: "var(--size-page-description)",
        lineHeight: 1.4,
        margin: 0,
        maxWidth: "44rem",
    },
    articleTitle: {
        fontFamily: tokens.textFont,
        fontSize: "var(--size-page-title)",
        fontWeight: 300,
        letterSpacing: "-0.035em",
        lineHeight: 1,
        margin: 0,
    },
    book: {
        alignContent: "start",
        display: "grid",
        gap: "0.75rem",
    },
    bookTitle: {
        alignItems: "center",
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "var(--size-label)",
        fontWeight: 600,
        letterSpacing: "0.02em",
        minHeight: tokens.publicationRow,
        ":hover": {
            color: tokens.accent,
        },
    },
    location: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem 1.5rem",
    },
    locationDate: {
        fontFamily: tokens.monoFont,
        fontSize: "var(--size-navigation)",
    },
    pagination: {
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontWeight: 600,
        gap: "1rem 2rem",
        justifyContent: "space-between",
        paddingTop: "1.25rem",
    },
});
