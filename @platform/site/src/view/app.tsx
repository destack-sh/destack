import "@destack/view/styles";
import "./site.css";
import { createRouter, defineRoutes } from "@destack/view/router";

import { Layout } from "./layout/layout";
import * as blog from "./page/blog";
import * as document from "./page/document";
import * as policy from "./page/policy";
import HomeIsland from "./islands/home";
import * as missing from "./page/missing";
import * as post from "./page/post";
import PricingIsland from "./islands/pricing";

/** The site's routes: every page inside the layout, the missing page last. */
const routes = defineRoutes([
    {
        component: Layout,
        children: [
            { path: "/", component: HomeIsland },
            { path: "/pricing", component: PricingIsland },
            { path: "/blog", component: blog.Blog },
            { path: "/blog/:slug", component: post.Post },
            { path: ["/docs", "/docs/*path"], component: document.Document },
            { path: "/legal/:slug", component: policy.Policy },
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
