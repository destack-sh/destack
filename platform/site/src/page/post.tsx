import { createMemo, Loading, Show } from "@destack/view";

import { loadPost, postBySlug, posts } from "../generated/posts";
import { BlogArticle } from "../reader/blog";
import { MissingPage } from "../site/missing";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

/** Properties for a blog post page. */
type PostPageProperties = {
    /** The requested post slug. */
    slug: string;
};

/** Render one generated blog post. */
export function PostPage(properties: PostPageProperties) {
    const post = () => postBySlug.get(properties.slug);
    const content = createMemo(() => loadPost(properties.slug));

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

                        <Loading>
                            <Show when={content()}>
                                {(content) => (
                                    <BlogArticle content={content()} post={post} posts={posts} />
                                )}
                            </Show>
                        </Loading>
                    </>
                )}
            </Show>
        </Shell>
    );
}

/** Render an unknown blog route. */
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
