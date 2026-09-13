import { A } from "@solidjs/router";
import { For } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { publicationStyles } from "../reader/publication.stylex";

import { posts, type Post } from "../generated/posts";
import { BlogNavigation } from "../reader/blog";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
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

            <section {...stylex.attrs(publicationStyles.layout)}>
                <aside {...stylex.attrs(publicationStyles.sidebar)}>
                    <BlogNavigation
                        activeHeading={() => ""}
                        contents={[]}
                        posts={orderedPosts}
                    />
                </aside>

                <div {...stylex.attrs(publicationStyles.article)}>
                    <header {...stylex.attrs(publicationStyles.header)}>
                        <h1 {...stylex.attrs(publicationStyles.title)}>
                            Articles
                        </h1>
                        <p {...stylex.attrs(publicationStyles.description)}>
                            Language design, runtime architecture, and
                            engineering notes from Destack.
                        </p>
                    </header>

                    <ol {...stylex.attrs(styles.archive)}>
                        <For each={orderedPosts}>
                            {(post) => <PostRow post={post} />}
                        </For>
                    </ol>
                </div>
            </section>
        </Shell>
    );
}

/// Properties for one archive row.
type PostRowProps = {
    /// The archive post.
    post: Post;
};

/// Render one archive row.
function PostRow(props: PostRowProps) {
    return (
        <li {...stylex.attrs(styles.postRow)}>
            <A {...stylex.attrs(styles.postLink)} href={props.post.route}>
                <time {...stylex.attrs(styles.date)}>{props.post.date}</time>

                <span {...stylex.attrs(styles.copy)}>
                    <strong {...stylex.attrs(styles.title)}>
                        {props.post.title}
                    </strong>
                    <span {...stylex.attrs(styles.subtitle)}>
                        {props.post.subtitle}
                    </span>
                </span>
            </A>
        </li>
    );
}

/// Sort newer posts before older posts.
function comparePosts(left: Post, right: Post) {
    return (
        right.date.localeCompare(left.date) ||
        left.title.localeCompare(right.title)
    );
}

const styles = stylex.create({
    archive: {
        listStyle: "none",
        margin: 0,
        padding: 0,
    },

    copy: {
        display: "grid",
        gap: tokens.publicationSpace,
        minWidth: 0,
    },
    date: {
        color: tokens.ink,
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        whiteSpace: "nowrap",
    },

    postLink: {
        alignItems: "baseline",
        display: "grid",
        gap: "1.5rem",
        gridTemplateColumns: "8rem minmax(0, 1fr)",
        paddingBlock: `calc(${tokens.publicationSpace} * 3)`,
        "@media (max-width: 640px)": {
            gap: tokens.publicationSpace,
            gridTemplateColumns: "minmax(0, 1fr)",
        },
        ":hover": {
            color: tokens.accent,
            backgroundColor: tokens.creamDeep,
        },
    },
    postRow: {
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
    },
    subtitle: {
        color: tokens.ink,
        lineHeight: 1.5,
    },
    title: {
        color: "inherit",
        fontFamily: tokens.displayFont,
        fontSize: "var(--size-subsection-title)",
        fontWeight: 500,
        lineHeight: 1.25,
    },
});
