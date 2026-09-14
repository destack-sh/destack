import { A } from "@solidjs/router";
import { type Accessor, For, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { publicationStyles } from "./publication.stylex";

import { type Post, type PostContent } from "../generated/posts";
import { ContentsTree, type ContentsEntry } from "./contents";
import { Breadcrumbs } from "./breadcrumbs";
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
            location={() => (
                <Breadcrumbs items={[{ href: "/blog/", label: "Blog" }]} />
            )}
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
            <A
                {...stylex.attrs(publicationStyles.collectionTitle)}
                href="/blog/"
            >
                Blog
            </A>

            <ol {...stylex.attrs(publicationStyles.collectionList)}>
                <For each={props.posts}>
                    {(post) => (
                        <li>
                            <A
                                {...stylex.attrs(
                                    styles.bookLink,
                                    post.route === props.current?.route &&
                                        publicationStyles.active,
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
        <header {...stylex.attrs(publicationStyles.header)}>
            <h1 {...stylex.attrs(publicationStyles.title)}>
                {props.post.title}
            </h1>
            <p {...stylex.attrs(publicationStyles.description)}>
                {props.post.subtitle}
            </p>
            <p {...stylex.attrs(styles.byline)}>
                <span>{props.post.author}</span>
                <time dateTime={props.post.date}>{props.post.date}</time>
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
    byline: {
        display: "flex",
        flexWrap: "wrap",
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        gap: "0.5rem 1.5rem",
        margin: "0.5rem 0 0",
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
        paddingTop: "1.25rem",
    },
    paginationLink: {
        color: tokens.ink,
        ":hover": {
            color: tokens.accent,
        },
    },
});
