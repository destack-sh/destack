import { createRouter, defineRoutes } from "@destack/view/router";
import { Loading } from "@destack/view";
import Home from "./routes/index.tsx";
import Blog from "./routes/blog/index.tsx";
import Post from "./routes/blog/[slug].tsx";
import Docs from "./routes/docs/index.tsx";
import Document from "./routes/docs/[...path].tsx";
import Missing from "./routes/[...404].tsx";

import "./style/site.css";
import "./style/content.css";

/** Public website routes. */
const routes = defineRoutes([
    { path: "/", component: Home },
    { path: "/blog", component: Blog },
    { path: "/blog/:slug", component: Post },
    { path: "/docs", component: Docs },
    { path: "/docs/*path", component: Document },
    { path: "*path", component: Missing },
]);

/** Website navigation with browser history and server request matching. */
const Router = createRouter({ routes });

/** Render the requested page. */
export default function App() {
    return <Router>{(props) => <Loading>{props.children}</Loading>}</Router>;
}
