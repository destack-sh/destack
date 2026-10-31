import { siteEndpoints } from "@destack/view/static";

/** The site's pages. */
export const paths = ["/", "/about/"];

/** The path rendering the not-found page. */
export const notFound = "/missing";

/** The site's sitemap of its pages. */
export const endpoints = siteEndpoints({ sitemap: paths });
