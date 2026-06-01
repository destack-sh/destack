import { A } from "@solidjs/router";
import { Show } from "solid-js";

import { BlogArticle } from "../component/blog-article";
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
                    <>
                        <Seo
                            description={post.subtitle}
                            path={post.route}
                            title={post.title}
                            type="article"
                        />

                        <BlogArticle post={post} posts={orderedPosts} />
                    </>
                )}
            </Show>
        </Shell>
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
