import { createRouter, defineRoutes } from "@destack/view/router";

import { Layout } from "./layout/layout";
import * as blog from "./page/blog";
import * as document from "./page/document";
import * as home from "./page/home";
import * as missing from "./page/missing";
import * as post from "./page/post";

/** The site's routes: every page inside the layout, the missing page last. */
const routes = defineRoutes([
    {
        component: Layout,
        children: [
            { path: "/", component: home.Home },
            { path: "/blog", component: blog.Blog },
            { path: "/blog/:slug", component: post.Post, preload: post.preload },
            {
                path: ["/docs", "/docs/*path"],
                component: document.Document,
                preload: document.preload,
            },
            { path: "*path", component: missing.Missing },
        ],
    },
]);

/** Website navigation with browser history and server request matching. */
const Router = createRouter({ routes });

/** Render the requested page. */
export default function App() {
    return <Router />;
}
