export { GraphIndexObject } from "../graph.ts";

/** Answer nothing at the Worker itself: the test reaches the index objects directly. */
export default {
    fetch(): Response {
        return new Response(null, { status: 404 });
    },
};
