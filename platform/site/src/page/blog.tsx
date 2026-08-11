import { A } from "@solidjs/router";
import { For } from "solid-js";

import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
import { posts, type Post } from "../generated/posts";

/// Render the blog archive.
export function BlogPage() {
    const orderedPosts = [...posts].sort(comparePosts);

    return (
        <Shell>
            <Seo
                description={
                    "Language design, runtime architecture, infrastructure, releases, " +
                    "and engineering notes from Destack."
                }
                path="/blog/"
                title="Blog"
            />

            <section class="blog-index">
                <header class="blog-index__header">
                    <h1 class="display">Blog</h1>
                    <span>
                        {orderedPosts.length} {orderedPosts.length === 1 ? "post" : "posts"}
                    </span>
                </header>

                <ol class="blog-archive">
                    <For each={orderedPosts}>{(post) => <PostRow post={post} />}</For>
                </ol>
            </section>
        </Shell>
    );
}

type PostRowProps = {
    /// The archive post.
    post: Post;
};

/// Render one archive row.
function PostRow(props: PostRowProps) {
    return (
        <li>
            <A href={props.post.route}>
                <time>{props.post.date}</time>

                <span class="blog-archive__copy">
                    <strong>{props.post.title}</strong>
                    <span>{props.post.subtitle}</span>
                </span>
            </A>
        </li>
    );
}

/// Sort newer posts before older posts.
function comparePosts(left: Post, right: Post) {
    return right.date.localeCompare(left.date) || left.title.localeCompare(right.title);
}
