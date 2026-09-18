import { useParams } from "@destack/view/router";

import { PostPage } from "../../page/post";

export default function BlogPost() {
    const params = useParams();

    return <PostPage slug={params.slug ?? ""} />;
}
