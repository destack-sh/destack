import { A } from "@solidjs/router";
import { Show } from "solid-js";

import { Seo } from "../component/seo";
import { Shell } from "../component/shell";
import { postBySlug, posts, type Post } from "../generated/posts";

type PostPageProps = {
    slug: string;
};

export function PostPage(props: PostPageProps) {
    const post = () => postBySlug.get(props.slug);
    const orderedPosts = [...posts].sort(comparePosts);

    return (
        <Shell>
            <Show keyed fallback={<MissingPost />} when={post()}>
                {(post) => (
                    <article class="mx-auto grid w-full max-w-[52rem] gap-10 px-4 py-8 md:px-10 md:py-12">
                        <Seo
                            description={post.summary}
                            path={post.route}
                            title={post.title}
                            type="article"
                        />

                        <header class="grid w-full gap-4">
                            <A
                                class="w-max border-b-4 border-neutral-300 text-sm font-extrabold lowercase hover:border-destack-accent"
                                href="/blog/"
                            >
                                blog
                            </A>

                            <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm font-extrabold text-neutral-500 lowercase">
                                <time>{post.date}</time>
                                <span>·</span>
                                <span>{post.author}</span>
                                <span>·</span>
                                <span>{post.status}</span>
                            </div>

                            <h1 class="page-title">{post.title}</h1>

                            <p class="text-base leading-7 font-bold text-neutral-700">
                                {post.summary}
                            </p>
                        </header>

                        <div class="grid w-full">
                            <div class="blog-prose min-w-0" innerHTML={post.html} />
                        </div>

                        <div class="w-full">
                            <PostNavigation post={post} posts={orderedPosts} />
                        </div>
                    </article>
                )}
            </Show>
        </Shell>
    );
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

function comparePosts(left: Post, right: Post) {
    return right.date.localeCompare(left.date) || left.title.localeCompare(right.title);
}

function MissingPost() {
    return (
        <section class="mx-auto grid w-full max-w-328 gap-4 px-4 py-12 md:px-10">
            <p class="text-sm font-extrabold text-destack-accent lowercase">missing post</p>
            <h1 class="page-title">this post does not exist</h1>
            <A
                class="w-max border-b-4 border-neutral-300 text-sm font-extrabold lowercase hover:border-destack-accent"
                href="/blog/"
            >
                back to blog
            </A>
        </section>
    );
}
