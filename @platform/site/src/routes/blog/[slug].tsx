import { useParams } from "@destack/view/router";

import { PostPage } from "../../page/post";

/** Render the blog post route. */
export default function BlogPost() {
    const parameters = useParams();

    return <PostPage slug={parameters.slug!} />;
}
