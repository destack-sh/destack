import { A } from "@solidjs/router";
import { For, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { type Post, type PostContent } from "../generated/posts";
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
            location={() => <BlogLocation post={props.post} />}
            navigation={() => <BlogNavigation current={props.post} posts={props.posts} />}
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
            <Breadcrumbs items={[{ href: "/blog/", label: "blog" }]} />
            <span>
                <time {...stylex.attrs(styles.locationDate)}>{props.post.date}</time>
                {" / "}{props.post.author}
            </span>
        </div>
    );
}

/// Properties for the blog collection navigation.
type BlogNavigationProps = {
    /// The current post.
    current: Post;

    /// Every post in reverse chronological order.
    posts: readonly Post[];
};

/// Render the blog collection beside an article.
function BlogNavigation(props: BlogNavigationProps) {
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
                                    post.slug === props.current.slug && styles.active,
                                )}
                                href={post.route}
                            >
                                <time {...stylex.attrs(styles.date)}>{post.date.slice(0, 4)}</time>
                                <span>{post.title}</span>
                            </A>
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
    active: {
        color: tokens.text,
        fontWeight: 600,
    },
    articleHeader: {
        display: "grid",
        gap: "0.6rem",
        marginTop: "0.9rem",
    },
    articleSubtitle: {
        color: tokens.soft,
        fontSize: "clamp(1rem, 1.4vw, 1.12rem)",
        lineHeight: 1.4,
        margin: 0,
        maxWidth: "44rem",
    },
    articleTitle: {
        fontFamily: tokens.monoFont,
        fontSize: "clamp(1.85rem, 4vw, 2.3rem)",
        fontWeight: 600,
        letterSpacing: "-0.025em",
        lineHeight: 1.1,
        margin: 0,
    },
    book: {
        alignContent: "start",
        display: "grid",
        gap: "0.75rem",
    },
    bookLink: {
        color: tokens.soft,
        display: "grid",
        fontSize: "0.78rem",
        gap: "0.1rem",
        ":hover": {
            color: tokens.text,
        },
    },
    bookList: {
        display: "grid",
        gap: "0.4rem",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    bookTitle: {
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        fontWeight: 700,
        letterSpacing: "0.06em",
        textTransform: "uppercase",
        width: "max-content",
        ":hover": {
            color: tokens.accent,
        },
    },
    date: {
        fontFamily: tokens.monoFont,
        fontSize: "0.72rem",
    },
    location: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem 1.5rem",
    },
    locationDate: {
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
    },
    pagination: {
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontWeight: 600,
        gap: "1rem 2rem",
        justifyContent: "space-between",
        paddingTop: "1.25rem",
    },
});
