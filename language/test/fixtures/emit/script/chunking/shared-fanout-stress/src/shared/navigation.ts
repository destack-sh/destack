import { getRoutePrefix } from "./routes.ts";

export function getPrimaryNavigation(activeSlug: string) {
    const slugs = ["overview", "admin", "docs", "reports"];

    return slugs.map((slug) => ({
        slug,
        href: `${getRoutePrefix()}/${slug}`,
        isActive: slug === activeSlug,
    }));
}
