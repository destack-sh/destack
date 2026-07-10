import { A } from "@solidjs/router";
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";

import { type Post, type TableOfContentsEntry } from "../generated/posts";

type BlogArticleProps = {
    /// The current post.
    post: Post;

    /// Every post in reverse chronological order.
    posts: readonly Post[];
};

/// Render a blog post and its navigation.
export function BlogArticle(props: BlogArticleProps) {
    return (
        <div class="blog-layout">
            <TableOfContents entries={props.post.tableOfContents} />

            <article class="blog-article">
                <BlogArticleHeader post={props.post} />
                <div class="blog-prose" innerHTML={props.post.html} />
                <PostNavigation post={props.post} posts={props.posts} />
            </article>
        </div>
    );
}

type BlogArticleHeaderProps = {
    /// The current post.
    post: Post;
};

/// Render the post title and metadata.
function BlogArticleHeader(props: BlogArticleHeaderProps) {
    return (
        <header class="blog-article__header">
            <div class="blog-article__eyebrow">
                <A href="/blog/">[blog]</A>
                <span>
                    <time>{props.post.date}</time> / {props.post.author}
                </span>
            </div>

            <h1>{props.post.title}</h1>
            <p>{props.post.subtitle}</p>
        </header>
    );
}

type TableOfContentsProps = {
    /// The post headings.
    entries: readonly TableOfContentsEntry[];
};

/// Render the active post outline on wide screens.
function TableOfContents(props: TableOfContentsProps) {
    const activeId = activeHeading(props.entries);

    return (
        <Show when={props.entries.length > 0}>
            <nav aria-label="contents" class="blog-contents">
                <p>[contents]</p>
                <ol>
                    <For each={props.entries}>
                        {(entry) => (
                            <li classList={{ "blog-contents__nested": entry.depth > 2 }}>
                                <a
                                    classList={{ "blog-contents__active": activeId() === entry.id }}
                                    href={`#${entry.id}`}
                                >
                                    {entry.text}
                                </a>
                            </li>
                        )}
                    </For>
                </ol>
            </nav>
        </Show>
    );
}

/// Track the last heading above the reading position.
function activeHeading(entries: readonly TableOfContentsEntry[]) {
    const [activeId, setActiveId] = createSignal(entries[0]?.id ?? "");

    onMount(() => {
        let frame = 0;

        // update at most once per rendered frame
        const update = () => {
            frame = 0;
            setActiveId(visibleHeading(entries));
        };

        const schedule = () => {
            if (frame === 0) {
                frame = window.requestAnimationFrame(update);
            }
        };

        update();
        window.addEventListener("scroll", schedule, { passive: true });
        window.addEventListener("resize", schedule);

        onCleanup(() => {
            if (frame !== 0) {
                window.cancelAnimationFrame(frame);
            }

            window.removeEventListener("scroll", schedule);
            window.removeEventListener("resize", schedule);
        });
    });

    return activeId;
}

/// Find the last heading above the top navigation.
function visibleHeading(entries: readonly TableOfContentsEntry[]) {
    const offset = 96;
    let current = entries[0]?.id ?? "";

    for (const entry of entries) {
        const element = document.getElementById(entry.id);
        if (element == undefined) {
            continue;
        }

        if (element.getBoundingClientRect().top > offset) {
            break;
        }

        current = entry.id;
    }

    return current;
}

type PostNavigationProps = {
    /// The current post.
    post: Post;

    /// Every post in reverse chronological order.
    posts: readonly Post[];
};

/// Render adjacent posts when they exist.
function PostNavigation(props: PostNavigationProps) {
    const index = () => props.posts.findIndex((post) => post.slug === props.post.slug);
    const newer = () => props.posts[index() - 1];
    const older = () => props.posts[index() + 1];

    return (
        <nav aria-label="post navigation" class="blog-post-navigation">
            <Show when={newer()}>
                {(post) => <PostNavigationLink label="newer" post={post()} />}
            </Show>

            <Show when={older()}>
                {(post) => <PostNavigationLink label="older" post={post()} />}
            </Show>
        </nav>
    );
}

type PostNavigationLinkProps = {
    /// The link label.
    label: string;

    /// The adjacent post.
    post: Post;
};

/// Render one adjacent post link.
function PostNavigationLink(props: PostNavigationLinkProps) {
    return (
        <A href={props.post.route}>
            [{props.label}] {props.post.title}
        </A>
    );
}
