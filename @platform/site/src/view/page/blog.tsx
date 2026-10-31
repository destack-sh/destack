import * as style from "@destack/style";
import { Metadata } from "@destack/view/document";

import { blogIndex, posts } from "../content/collection";
import Archive from "../islands/archive";
import Directory from "../islands/directory";
import { publicationStyles } from "../reader/publication.stylex";
import { Reader } from "../reader/reader";

/** Browse articles with the shared collection layout. */
export function Blog() {
    const entries = posts.map((post) => ({
        title: post.title,
        href: post.route,
        summary: post.subtitle,
        date: post.date,
        cover: post.cover,
    }));

    return (
        <>
            <Metadata
                title="Blog"
                description="Notes from building Destack."
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
                        <Archive entries={entries} />
                    </nav>
                )}
                publication="journal"
                source={blogIndex}
            >
                <Directory
                    title="Blog"
                    description="Notes from building Destack."
                    entries={entries}
                />
            </Reader>
        </>
    );
}
