import { expect, test } from "@destack/test";
import {
    Breadcrumb,
    BreadcrumbEllipsis,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** The markup of an icon whose body is left out. */
const ICON =
    '<svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true"></svg>';

test("render a breadcrumb trail ending at the current page", () => {
    const container = draw(() => (
        <Breadcrumb>
            <BreadcrumbList>
                <BreadcrumbItem>
                    <BreadcrumbLink href="/">Home</BreadcrumbLink>
                </BreadcrumbItem>
                <BreadcrumbSeparator />
                <BreadcrumbItem>
                    <BreadcrumbEllipsis />
                </BreadcrumbItem>
                <BreadcrumbSeparator />
                <BreadcrumbItem>
                    <BreadcrumbPage>Groceries</BreadcrumbPage>
                </BreadcrumbItem>
            </BreadcrumbList>
        </Breadcrumb>
    ));
    const separator = `<li role="presentation" aria-hidden="true" data-slot="breadcrumb-separator">${ICON}</li>`;
    expect(markup(container)).toBe(
        '<nav aria-label="Breadcrumb" data-slot="breadcrumb"><ol data-slot="breadcrumb-list">' +
            '<li data-slot="breadcrumb-item"><a data-slot="breadcrumb-link" href="/">Home</a></li>' +
            separator +
            `<li data-slot="breadcrumb-item"><span data-slot="breadcrumb-ellipsis">${ICON}<span>More</span></span></li>` +
            separator +
            '<li data-slot="breadcrumb-item"><span aria-current="page" data-slot="breadcrumb-page">Groceries</span></li></ol></nav>',
    );
});
