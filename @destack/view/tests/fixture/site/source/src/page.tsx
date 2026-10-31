import { createSignal, httpStatus } from "@destack/view";
import { createRouter, defineRoutes } from "@destack/view/router";
import logo from "./logo.png";

/** Render a page's heading with a counter that hydrates. */
function Page(properties: { readonly title: string }) {
    const [count, setCount] = createSignal(0);

    return (
        <main>
            <h1>{properties.title}</h1>
            <img alt="" src={logo.src} />
            <button onClick={() => setCount(count() + 1)}>Count: {count()}</button>
        </main>
    );
}

/** Render the missing page, answering not found. */
function Missing() {
    httpStatus(404);

    return <Page title="Missing" />;
}

/** The site's routes: two pages, and the missing page for every other path. */
const routes = defineRoutes([
    { path: "/", component: () => <Page title="Home" /> },
    { path: "/about", component: () => <Page title="About" /> },
    { path: "*path", component: Missing },
]);

/** Site navigation with browser history and server request matching. */
const Router = createRouter({ routes });

/** Render the requested page. */
export default function App() {
    return <Router />;
}
