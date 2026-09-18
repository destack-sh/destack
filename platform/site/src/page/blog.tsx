import { blogIndex, posts } from "../generated/posts";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
import { Reader } from "../reader/reader";
import { createDirectory, DirectoryArchive, DirectoryContent } from "../reader/directory";

/// Browse articles with the shared collection layout.
export function BlogPage() {
    const entries = [...posts].sort((left, right) =>
        right.date.localeCompare(left.date) || left.title.localeCompare(right.title)
    )
        .map((post) => ({
            title: post.title,
            href: post.route,
            summary: post.subtitle,
            date: post.date,
        }));
    const directory = createDirectory(() => entries);

    return (
        <Shell>
            <Seo
                description="Language design, runtime architecture, infrastructure, releases, and engineering notes from Destack."
                path="/blog/"
                title="Blog"
            />
            <Reader
                location={() => null}
                navigation={() => (
                    <nav aria-label="Blog archive">
                        <div class="collection-context">
                            <a href="/blog/">Blog</a>
                        </div>
                        <div class="directory-filters">
                            <DirectoryArchive directory={directory} />
                        </div>
                    </nav>
                )}
                publication="journal"
                source={blogIndex}
            >
                <DirectoryContent title="Blog" directory={directory} />
            </Reader>
        </Shell>
    );
}
