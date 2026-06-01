import { A } from "@solidjs/router";
import { For, Show } from "solid-js";

import { Panel } from "../component/panel";
import { Seo } from "../component/seo";
import { Shell } from "../component/shell";
import { posts, type Post } from "../generated/posts";

export function BlogPage() {
    const orderedPosts = [...posts].sort(comparePosts);
    const latestPost = orderedPosts[0];

    return (
        <Shell>
            <Seo
                description="Language design, runtime architecture, infrastructure, releases, and engineering notes from Destack."
                path="/blog/"
                title="Blog"
            />
            <section class="mx-auto grid w-full max-w-[52rem] gap-10 px-4 py-8 md:px-10 md:py-12">
                <Show when={latestPost}>
                    {(post) => <LatestPost post={post()} posts={orderedPosts} />}
                </Show>

                <Archive posts={orderedPosts} />
            </section>
        </Shell>
    );
}

type PostRowProps = {
    post: Post;
};

type LatestPostProps = {
    post: Post;
    posts: readonly Post[];
};

function LatestPost(props: LatestPostProps) {
    return (
        <article class="grid w-full gap-8">
            <header class="grid w-full gap-5">
                <A
                    class="w-max border-b-4 border-neutral-300 text-sm font-extrabold lowercase hover:border-destack-accent"
                    href={props.post.route}
                >
                    latest
                </A>

                <PostMeta post={props.post} />

                <h1 class="page-title mb-1">{props.post.title}</h1>
                <p class="text-xl leading-8 font-black text-neutral-700">
                    {props.post.subtitle}
                </p>
                <PostTags post={props.post} />
            </header>

            <div class="blog-prose min-w-0" innerHTML={props.post.html} />

            <PostNavigation post={props.post} posts={props.posts} />
        </article>
    );
}

type ArchiveProps = {
    posts: readonly Post[];
};

function Archive(props: ArchiveProps) {
    return (
        <Panel class="w-full" depth="deep" title="blog">
            <ol class="grid px-5 py-4">
                <For each={props.posts}>
                    {(post) => <PostRow post={post} />}
                </For>
            </ol>
        </Panel>
    );
}

function PostRow(props: PostRowProps) {
    return (
        <li class="border-b-2 border-neutral-950">
            <A
                class="grid gap-3 py-4 hover:bg-white md:grid-cols-[minmax(0,1fr)_auto] md:items-baseline md:px-3"
                href={props.post.route}
            >
                <span class="grid gap-1">
                    <span class="text-base leading-6 font-black md:text-lg">
                        {props.post.title}
                    </span>
                    <span class="max-w-2xl text-sm leading-5 font-extrabold text-neutral-500 lowercase">
                        {props.post.subtitle}
                    </span>
                </span>

                <time class="text-sm font-extrabold whitespace-nowrap text-neutral-500">
                    {readableDate(props.post.date)}
                </time>
            </A>
        </li>
    );
}

type PostNavigationProps = {
    post: Post;
    posts: readonly Post[];
};

function PostNavigation(props: PostNavigationProps) {
    const index = () => props.posts.findIndex((post) => post.slug === props.post.slug);
    const older = () => props.posts[index() + 1];

    return (
        <Show when={older()}>
            {(post) => (
                <A
                    class="grid gap-1 border-t-2 border-neutral-950 pt-5 hover:text-destack-accent"
                    href={post().route}
                >
                    <span class="text-sm font-extrabold text-neutral-500 lowercase">older</span>
                    <span class="text-base font-black">{post().title}</span>
                </A>
            )}
        </Show>
    );
}

function PostMeta(props: PostRowProps) {
    return (
        <p class="flex flex-wrap gap-x-3 gap-y-1 text-sm font-extrabold text-neutral-500 lowercase">
            <time>{props.post.date}</time>
            <span>·</span>
            <span>{props.post.author}</span>
        </p>
    );
}

function PostTags(props: PostRowProps) {
    return (
        <span class="flex flex-wrap gap-2">
            <For each={props.post.tags}>
                {(tag) => (
                    <span class="text-xs font-extrabold text-neutral-500 lowercase">#{tag}</span>
                )}
            </For>
        </span>
    );
}

function comparePosts(left: Post, right: Post) {
    return right.date.localeCompare(left.date) || left.title.localeCompare(right.title);
}

function readableDate(date: string) {
    const parsed = new Date(`${date}T00:00:00Z`);

    return new Intl.DateTimeFormat("en", {
        day: "2-digit",
        month: "short",
        timeZone: "UTC",
        year: "numeric",
    }).format(parsed);
}
