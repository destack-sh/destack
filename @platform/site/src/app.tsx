import { createRouter, defineRoutes } from "@destack/view/router";
import { Loading } from "@destack/view";
import Home from "./routes/home.tsx";
import Blog from "./routes/blog/blog.tsx";
import Post from "./routes/blog/[slug].tsx";
import Documentation from "./routes/docs/documentation.tsx";
import Document from "./routes/docs/[...path].tsx";
import Missing from "./routes/[...404].tsx";

import "./style/site.css";
import "./style/content.css";

/** Public website routes. */
const routes = defineRoutes([
    { path: "/", component: Home },
    { path: "/blog", component: Blog },
    { path: "/blog/:slug", component: Post },
    { path: "/docs", component: Documentation },
    { path: "/docs/*path", component: Document },
    { path: "*path", component: Missing },
]);

/** Website navigation with browser history and server request matching. */
const Router = createRouter({ routes });

/** Render the requested page. */
export default function App() {
    return <Router>{(properties) => <Loading>{properties.children}</Loading>}</Router>;
}
