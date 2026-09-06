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

            <section {...stylex.attrs(styles.index)}>
                <aside {...stylex.attrs(styles.sidebar)}>
                    <BlogNavigation
                        activeHeading={() => ""}
                        contents={[]}
                        posts={orderedPosts}
                    />
                </aside>

                <div {...stylex.attrs(styles.article)}>
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

const narrow = "@media (width < 60rem)";
const mobile = "@media (max-width: 767px)";

const styles = stylex.create({
    archive: {
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    article: {
        alignContent: "start",
        display: "grid",
        gridColumn: 2,
        minWidth: 0,
        width: "100%",
        [narrow]: {
            gridColumn: "auto",
        },
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

    index: {
        columnGap: "4rem",
        display: "grid",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-body)",
        gridTemplateColumns: "15rem minmax(0, 1fr)",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        padding: `1.5rem ${tokens.gutterRight} 4rem ${tokens.gutterLeft}`,
        width: "100%",
        [narrow]: {
            display: "block",
            maxWidth: "48rem",
            padding: `1rem ${tokens.gutterRight} 3rem ${tokens.gutterLeft}`,
        },
        [mobile]: {
            paddingTop: "0.75rem",
        },
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
        },
    },
    postRow: {
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
    },
    sidebar: {
        alignContent: "start",
        alignSelf: "start",
        display: "none",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        gridColumn: 1,
        gap: 0,
        "@media (min-width: 60rem)": {
            display: "grid",
            position: "sticky",
            top: "1.5rem",
        },
    },
    subtitle: {
        color: tokens.ink,
        lineHeight: 1.5,
    },
    title: {
        color: tokens.ink,
        fontSize: "var(--size-minor-title)",
        fontWeight: 600,
        lineHeight: 1.25,
    },
});
