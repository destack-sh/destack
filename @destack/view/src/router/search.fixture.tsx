import { useBlocker } from "./blocker.ts";
import { createRouter, defineRoutes, useLocation } from "./index.ts";
import { parseAsInteger, parseAsString, useQueryStates } from "./search.ts";

/** The URL state of the fixture's list of notes. */
const notes = {
    query: parseAsString.withDefault(""),
    page: parseAsInteger.withDefault(1),
};

/** Show the list's URL state, the address it is at and whether leaving is held. */
function Page() {
    // follow the URL state, the address and leaving
    const [search] = useQueryStates(notes, { urlKeys: { query: "q" } });
    const location = useLocation();
    const blocker = useBlocker(false);

    return (
        <p>
            {search().query} {search().page} {location.search} {blocker.state()}
        </p>
    );
}

/** The fixture's router over every path. */
const Router = createRouter({ routes: defineRoutes([{ path: "*path", component: Page }]) });

/** Render the fixture at an address with an invalid page, on the server and in the browser alike. */
export function NotesAtInvalidPage() {
    return <Router url="/notes?q=milk&page=x" />;
}
