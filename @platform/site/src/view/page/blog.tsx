import * as style from "@destack/style";
import { Metadata } from "@destack/view/document";

import { blogIndex, posts } from "../content/generated/posts";
import { createDirectory, DirectoryArchive, DirectoryContent } from "../reader/directory";
import { publicationStyles } from "../reader/publication.stylex";
import { Reader } from "../reader/reader";

/** Browse articles with the shared collection layout. */
export function Blog() {
    const entries = posts.map((post) => ({
        title: post.title,
        href: post.route,
        summary: post.subtitle,
        date: post.date,
        image: post.cover,
    }));
    const directory = createDirectory(() => entries);

    return (
        <>
            <Metadata
                title="Blog"
                description="Updates around Destack."
                alternates={{
                    canonical: "/blog/",
                    types: { "application/atom+xml": "/blog/feed.xml" },
                }}
            />
            <Reader
                location={() => null}
                navigation={() => (
                    <nav aria-label="Blog archive">
                        <div {...style.attrs(publicationStyles.context)}>
                            <a href="/blog/" {...style.attrs(publicationStyles.contextTitle)}>
                                Blog
                            </a>
                        </div>
                        <DirectoryArchive directory={directory} />
                    </nav>
                )}
                publication="journal"
                source={blogIndex}
            >
                <DirectoryContent
                    title="Blog"
                    description="Updates around Destack."
                    directory={directory}
                />
            </Reader>
        </>
    );
}
