import { renderInsights } from "./summary.ts";

type PageState = {
    slug: string;
    navigation: { slug: string; href: string; isActive: boolean }[];
    section: { heading: string; cards: string[] };
    insights: string[];
};

export function renderShell(page: PageState) {
    const navigation = page.navigation
        .map((item) => `${item.slug}:${item.href}:${item.isActive}`)
        .join("|");
    const cards = page.section.cards.join(",");

    return `${page.slug}:${navigation}:${page.section.heading}:${cards}:${renderInsights(page.insights)}`;
}
