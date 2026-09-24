import * as stylex from "@destack/style";

import { blogIndex, posts } from "../generated/posts";
import { createDirectory, DirectoryArchive, DirectoryContent } from "../reader/directory";
import { publicationStyles } from "../reader/publication.stylex";
import { Reader } from "../reader/reader";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

/** Browse articles with the shared collection layout. */
export function BlogPage() {
    const entries = posts.map((post) => ({
        title: post.title,
        href: post.route,
        summary: post.subtitle,
        date: post.date,
        image: post.cover,
    }));
    const directory = createDirectory(() => entries);

    return (
        <Shell>
            <Seo description="Updates around Destack." path="/blog/" title="Blog" />
            <Reader
                location={() => null}
                navigation={() => (
                    <nav aria-label="Blog archive">
                        <div {...stylex.attrs(publicationStyles.context)}>
                            <a href="/blog/" {...stylex.attrs(publicationStyles.contextTitle)}>
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
        </Shell>
    );
}
