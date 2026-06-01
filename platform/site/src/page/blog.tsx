import { A } from "@solidjs/router";
import { For, Show } from "solid-js";

import { BlogArticle } from "../component/blog-article";
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
            <Show when={latestPost}>
                {(post) => (
                    <BlogArticle post={post()} posts={orderedPosts}>
                        <Archive posts={orderedPosts} />
                    </BlogArticle>
                )}
            </Show>
        </Shell>
    );
}

type PostRowProps = {
    post: Post;
};

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
