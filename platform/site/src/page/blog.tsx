import { A } from "@solidjs/router";
import { For } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
import { posts, type Post } from "../generated/posts";
import { tokens } from "../style/tokens.stylex";

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

const styles = stylex.create({
    archive: {
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    copy: {
        display: "grid",
        gap: "0.5rem",
        minWidth: 0,
    },
    date: {
        color: tokens.soft,
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        whiteSpace: "nowrap",
    },
    header: {
        alignItems: "baseline",
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: "2px",
        display: "flex",
        gap: "1.5rem",
        justifyContent: "space-between",
        paddingBlock: "1rem",
    },
    heading: {
        fontFamily: tokens.textFont,
        fontSize: "clamp(2rem, 5vw, 2.75rem)",
        fontWeight: 500,
        letterSpacing: "-0.025em",
        lineHeight: 1,
        margin: 0,
    },
    index: {
        fontFamily: tokens.textFont,
        fontSize: "1rem",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        padding: `3.5rem ${tokens.gutterRight} 5rem ${tokens.gutterLeft}`,
        width: "100%",
    },
    postCount: {
        color: tokens.soft,
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
    },
    postLink: {
        alignItems: "baseline",
        display: "grid",
        gap: "1.5rem",
        gridTemplateColumns: "10rem minmax(0, 1fr)",
        paddingBlock: "1.5rem",
        "@media (max-width: 640px)": {
            gap: "0.5rem",
            gridTemplateColumns: "minmax(0, 1fr)",
        },
    },
    postRow: {
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
    },
    postRowFirst: {
        borderTopWidth: 0,
    },
    subtitle: {
        color: tokens.soft,
        lineHeight: 1.5,
    },
    title: {
        fontFamily: tokens.textFont,
        fontSize: "1.25rem",
        fontWeight: 600,
        lineHeight: 1.25,
    },
});
