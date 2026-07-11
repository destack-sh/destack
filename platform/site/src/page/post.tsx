import { createResource, Show, Suspense } from "solid-js";

import { loadPost, postBySlug, posts, type Post } from "../generated/posts";
import { BlogArticle } from "../reader/blog";
import { MissingPage } from "../site/missing";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

type PostPageProps = {
    /// The requested post slug.
    slug: string;
};

/// Render one generated blog post.
export function PostPage(props: PostPageProps) {
    const post = () => postBySlug.get(props.slug);
    const orderedPosts = [...posts].sort(comparePosts);
    const [content] = createResource(() => props.slug, loadPost);

    return (
        <Shell>
            <Show keyed fallback={<MissingPost />} when={post()}>
                {(post) => (
                    <>
                        <Seo
                            description={post.subtitle}
                            markdownRoute={post.markdownRoute}
                            path={post.route}
                            textRoute={post.textRoute}
                            title={post.title}
                            type="article"
                        />

                        <Suspense>
                            <Show when={content()}>
                                {(content) => (
                                    <BlogArticle content={content()} post={post} posts={orderedPosts} />
                                )}
                            </Show>
                        </Suspense>
                    </>
                )}
            </Show>
        </Shell>
    );
}

/// Sort newer posts before older posts.
function comparePosts(left: Post, right: Post) {
    return right.date.localeCompare(left.date) || left.title.localeCompare(right.title);
}

/// Render an unknown blog route.
function MissingPost() {
    return (
        <MissingPage
            backHref="/blog/"
            backLabel="back to blog"
            description="This blog post does not exist."
            label="missing post"
            title="this post does not exist"
        />
    );
}
