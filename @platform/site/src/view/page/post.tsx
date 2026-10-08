import { present } from "@destack/schema";
import { createMemo, Show } from "@destack/view";
import { query, type RoutePreloadFuncArgs, type RouteSectionProps } from "@destack/view/router";
import { Metadata } from "@destack/view/document";

import { loadPost, type PostContent, postBySlug, posts } from "../content/generated/posts";
import { BlogArticle } from "../reader/blog";
import { MissingPage } from "./missing";

/** Load a post's rendered body once per slug, for the preload and the page alike. */
const content = query(loadPost, "post");

/** Load the requested post's body while the route is preloading. */
export function preload(arguments_: RoutePreloadFuncArgs): Promise<PostContent | undefined> {
    return content(present(arguments_.params["slug"], "a blog post slug"));
}

/** Render one blog post, or the missing post page for an unknown slug. */
export function Post(properties: RouteSectionProps<Promise<PostContent | undefined>>) {
    const post = () => postBySlug.get(present(properties.params["slug"], "a blog post slug"));
    const loaded = createMemo(() => properties.data);

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
