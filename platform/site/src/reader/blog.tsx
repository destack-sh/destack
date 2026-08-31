import { A } from "@solidjs/router";
import { type Accessor, For, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { type Post, type PostContent } from "../generated/posts";
import { ContentsTree, type ContentsEntry } from "./contents";
import { tokens } from "../style/tokens.stylex";
import { Reader } from "./reader";

const mobile = "@media (max-width: 767px)";

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
                    current={props.post}
                    posts={props.posts}
                />
            )}
            publication="journal"
            source={props.post}
            tokenCount={props.post.tokens}
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

/// Render the post publication details.
function BlogLocation(props: BlogLocationProps) {
    return (
        <span>
            <time>{props.post.date}</time>
            {" / "}
            {props.post.author}
        </span>
    );
}

/// Properties for the blog article navigation.
export type BlogNavigationProps = {
    /// The currently active heading identifier.
    activeHeading: Accessor<string>;

    /// The headings in the current article.
    contents: readonly ContentsEntry[];

    /// The current post when rendering an article.
    current?: Post;

    /// Every post in reverse chronological order.
    posts: readonly Post[];
};

/// Render the blog archive and current article headings.
export function BlogNavigation(props: BlogNavigationProps) {
    return (
        <nav aria-label="blog" {...stylex.attrs(styles.book)}>
            <A {...stylex.attrs(styles.bookTitle)} href="/blog/">
                blog
            </A>

            <ol {...stylex.attrs(styles.bookList)}>
                <For each={props.posts}>
                    {(post) => (
                        <li>
                            <A
                                {...stylex.attrs(
                                    styles.bookLink,
                                    post.route === props.current?.route &&
                                        styles.active,
                                )}
                                href={post.route}
                            >
                                {post.title}
                            </A>

                            <Show when={post.route === props.current?.route}>
                                <div {...stylex.attrs(styles.bookContents)}>
                                    <ContentsTree
                                        activeId={props.activeHeading}
                                        entries={props.contents}
                                        isNested
                                    />
                                </div>
                            </Show>
                        </li>
                    )}
                </For>
            </ol>
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
        <header {...stylex.attrs(styles.articleHeader)}>
            <h1 {...stylex.attrs(styles.articleTitle)}>{props.post.title}</h1>
            <p {...stylex.attrs(styles.articleSubtitle)}>
                {props.post.subtitle}
            </p>
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
    const index = () =>
        props.posts.findIndex((post) => post.slug === props.post.slug);
    const newer = () => props.posts[index() - 1];
    const older = () => props.posts[index() + 1];

    return (
        <nav aria-label="post navigation" {...stylex.attrs(styles.pagination)}>
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
    active: {
        color: tokens.ink,
        fontWeight: 600,
    },
    articleHeader: {
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "grid",
        gap: tokens.publicationSpace,
        paddingBlock: `calc(${tokens.publicationSpace} * 4)`,
        [mobile]: {
            gap: tokens.publicationSpace,
            paddingBlock: "1rem",
        },
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
        fontFamily: tokens.displayFont,
        fontSize: "var(--size-page-title)",
        fontWeight: 400,
        letterSpacing: "-0.03em",
        lineHeight: 1,
        margin: 0,
        textIndent: "-0.04em",
    },
    book: {
        alignContent: "start",
        display: "grid",
        gap: 0,
    },
    bookContents: {
        paddingBottom: `calc(${tokens.publicationSpace} * 1.5)`,
        paddingTop: `calc(${tokens.publicationSpace} * 0.5)`,
    },
    bookLink: {
        color: tokens.soft,
        display: "block",
        lineHeight: 1.3,
        paddingBlock: "0.25rem",
        ":hover": {
            color: tokens.accent,
        },
    },
    bookList: {
        display: "grid",
        gap: 0,
        listStyle: "none",
        margin: 0,
        padding: `calc(${tokens.publicationSpace} * 4) 0 0`,
    },
    bookTitle: {
        alignItems: "center",
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "flex",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-label)",
        fontWeight: 500,
        minHeight: tokens.publicationRow,
        ":hover": {
            color: tokens.accent,
        },
    },
    pagination: {
        borderTopColor: tokens.line,
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
    paginationLink: {
        color: tokens.ink,
        ":hover": {
            color: tokens.accent,
        },
    },
});
