import { A } from "@solidjs/router";
import { For } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
import { posts, type Post } from "../generated/posts";
import { styles } from "./blog.stylex";

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

            <section {...stylex.attrs(styles.index)}>
                <header {...stylex.attrs(styles.header)}>
                    <h1 {...stylex.attrs(styles.heading)}>Blog</h1>
                    <span {...stylex.attrs(styles.postCount)}>
                        {orderedPosts.length} {orderedPosts.length === 1 ? "post" : "posts"}
                    </span>
                </header>

                <ol {...stylex.attrs(styles.archive)}>
                    <For each={orderedPosts}>
                        {(post, index) => <PostRow isFirst={index() === 0} post={post} />}
                    </For>
                </ol>
            </section>
        </Shell>
    );
}

type PostRowProps = {
    /// Whether the post begins the archive.
    isFirst: boolean;

    /// The archive post.
    post: Post;
};

/// Render one archive row.
function PostRow(props: PostRowProps) {
    return (
        <li {...stylex.attrs(styles.postRow, props.isFirst && styles.postRowFirst)}>
            <A {...stylex.attrs(styles.postLink)} href={props.post.route}>
                <time {...stylex.attrs(styles.date)}>{props.post.date}</time>

                <span {...stylex.attrs(styles.copy)}>
                    <strong {...stylex.attrs(styles.title)}>{props.post.title}</strong>
                    <span {...stylex.attrs(styles.subtitle)}>{props.post.subtitle}</span>
                </span>
            </A>
        </li>
    );
}

/// Sort newer posts before older posts.
function comparePosts(left: Post, right: Post) {
    return right.date.localeCompare(left.date) || left.title.localeCompare(right.title);
}
