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
        <section class="blog-missing">
            <p>[missing post]</p>
            <h1>this post does not exist</h1>
            <A href="/blog/">[back to blog]</A>
        </section>
    );
}
