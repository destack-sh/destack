import { A } from "@solidjs/router";
import { For, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { type Post, type PostContent } from "../generated/posts";
import { Breadcrumbs } from "./breadcrumbs";
import { journalStyles } from "./publication.stylex";
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
        <div {...stylex.attrs(journalStyles.location)}>
            <Breadcrumbs items={[{ href: "/blog/", label: "blog" }]} />
            <span>
                <time {...stylex.attrs(journalStyles.locationDate)}>{props.post.date}</time>
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
        <nav aria-label="blog" {...stylex.attrs(journalStyles.book)}>
            <A {...stylex.attrs(journalStyles.bookTitle)} href="/blog/">
                blog
            </A>

            <ol {...stylex.attrs(journalStyles.bookList)}>
                <For each={props.posts}>
                    {(post) => (
                        <li>
                            <A
                                {...stylex.attrs(
                                    journalStyles.bookLink,
                                    post.slug === props.current.slug && journalStyles.active,
                                )}
                                href={post.route}
                            >
                                <time {...stylex.attrs(journalStyles.date)}>{post.date.slice(0, 4)}</time>
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
        <header {...stylex.attrs(journalStyles.articleHeader)}>
            <h1 {...stylex.attrs(journalStyles.articleTitle)}>{props.post.title}</h1>
            <p {...stylex.attrs(journalStyles.articleSubtitle)}>{props.post.subtitle}</p>
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
        <nav aria-label="post navigation" {...stylex.attrs(journalStyles.pagination)}>
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
