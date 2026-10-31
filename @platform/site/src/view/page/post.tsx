import { present } from "@destack/schema";
import { Show } from "@destack/view";
import type { RouteSectionProps } from "@destack/view/router";
import { Metadata } from "@destack/view/document";

import { postContent, posts } from "../content/collection";
import { BlogArticle } from "../reader/blog";
import { MissingPage } from "./missing";

/** Render one blog post, or the missing post page for an unknown slug. */
export function Post(properties: RouteSectionProps) {
    // find the post and its body by the slug
    const slug = () => present(properties.params["slug"], "a blog post slug");
    const post = () => posts.find((each) => each.slug === slug());
    const loaded = () => postContent.get(slug());

    return (
        <Show keyed fallback={<MissingPost />} when={post()}>
            {(found) => (
                <>
                    <Metadata
                        title={found.title}
                        description={found.subtitle}
                        authors={[{ name: found.author }]}
                        alternates={{
                            canonical: found.route,
                            types: {
                                "text/markdown": found.markdownRoute,
                                "text/plain": found.textRoute,
                            },
                        }}
                        openGraph={{
                            type: "article",
                            publishedTime: found.date,
                            authors: [found.author],
                        }}
                    />
                    <Show when={loaded()}>
                        {(body) => <BlogArticle content={body()} post={found} posts={posts} />}
                    </Show>
                </>
            )}
        </Show>
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
