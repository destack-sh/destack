import { A } from "@solidjs/router";
import { createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";

import { type Post, type TableOfContentsEntry } from "../generated/posts";

type BlogArticleProps = {
    children?: JSX.Element;
    post: Post;
    posts: readonly Post[];
};

export function BlogArticle(props: BlogArticleProps) {
    return (
        <div class="mx-auto grid w-full max-w-[96rem] grid-cols-[minmax(0,56rem)] gap-6 px-4 py-8 md:px-10 md:py-12 2xl:grid-cols-[18rem_minmax(0,56rem)_18rem] 2xl:items-start">
            <TableOfContents entries={props.post.tableOfContents} />

            <div class="relative isolate min-w-0 pr-2 pb-2">
                <div
                    aria-hidden="true"
                    class="pointer-events-none absolute top-3 right-0 bottom-0 left-3 z-0 bg-size-[3px_3px] bg-[radial-gradient(circle,var(--color-destack-accent)_0_1.15px,transparent_1.3px)]"
                />

                <article class="relative z-10 grid w-full gap-10 border-2 border-neutral-950 bg-destack-panel px-4 py-6 md:px-8 md:py-8">
                    <BlogArticleHeader post={props.post} />

                    <div class="grid w-full">
                        <div class="blog-prose min-w-0" innerHTML={props.post.html} />
                    </div>

                    <div class="w-full">
                        <PostNavigation post={props.post} posts={props.posts} />
                    </div>

                    {props.children}
                </article>
            </div>

            <div aria-hidden="true" class="hidden 2xl:block" />
        </div>
    );
}

type BlogArticleHeaderProps = {
    post: Post;
};

function BlogArticleHeader(props: BlogArticleHeaderProps) {
    return (
        <header class="grid w-full gap-5">
            <A
                class="w-max border-b-4 border-neutral-300 text-sm font-extrabold lowercase hover:border-destack-accent"
                href="/blog/"
            >
                blog
            </A>

            <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm font-extrabold text-neutral-500 lowercase">
                <time>{props.post.date}</time>
                <span>·</span>
                <span>{props.post.author}</span>
            </div>

            <h1 class="page-title mb-1">{props.post.title}</h1>
            <p class="text-xl leading-8 font-black text-neutral-700">{props.post.subtitle}</p>
        </header>
    );
}

type TableOfContentsProps = {
    entries: readonly TableOfContentsEntry[];
};

function TableOfContents(props: TableOfContentsProps) {
    const activeId = activeHeading(props.entries);

    return (
        <Show when={props.entries.length > 0}>
            <nav
                aria-label="contents"
                class="sticky top-20 hidden max-h-[calc(100svh-8rem)] overflow-auto pr-3 2xl:block"
            >
                <p class="mb-3 text-xs font-black tracking-normal text-neutral-500 lowercase">
                    contents
                </p>
                <ol class="grid gap-2 border-l-2 border-neutral-300 pl-3 text-xs leading-5 font-extrabold lowercase">
                    <For each={props.entries}>
                        {(entry) => (
                            <li
                                classList={{
                                    "pl-3": entry.depth > 2,
                                }}
                            >
                                <a
                                    class="block border-l-4 py-0.5 pl-2 underline decoration-2 underline-offset-4"
                                    classList={{
                                        "border-destack-accent text-neutral-950 decoration-destack-accent":
                                            activeId() === entry.id,
                                        "border-transparent text-neutral-500 decoration-neutral-300 hover:text-destack-accent hover:decoration-destack-accent":
                                            activeId() !== entry.id,
                                    }}
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

function activeHeading(entries: readonly TableOfContentsEntry[]) {
    const [activeId, setActiveId] = createSignal(entries[0]?.id ?? "");

    onMount(() => {
        let frame = 0;

        const update = () => {
            frame = 0;
            const current = visibleHeading(entries);
            setActiveId(current);
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
    post: Post;
    posts: readonly Post[];
};

function PostNavigation(props: PostNavigationProps) {
    const index = () => props.posts.findIndex((post) => post.slug === props.post.slug);
    const newer = () => props.posts[index() - 1];
    const older = () => props.posts[index() + 1];

    return (
        <nav class="grid gap-3 border-t-2 border-neutral-950 pt-5 md:grid-cols-2">
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
    label: string;
    post: Post;
};

function PostNavigationLink(props: PostNavigationLinkProps) {
    return (
        <A
            class="grid gap-1 border-2 border-neutral-950 bg-destack-panel p-4 hover:bg-white"
            href={props.post.route}
        >
            <span class="text-sm font-extrabold text-neutral-500 lowercase">{props.label}</span>
            <span class="text-base font-black">{props.post.title}</span>
        </A>
    );
}
