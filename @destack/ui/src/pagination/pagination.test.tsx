import { expect, test } from "@destack/test";
import {
    Pagination,
    PaginationContent,
    PaginationEllipsis,
    PaginationItem,
    PaginationLink,
    PaginationNext,
    PaginationPrevious,
} from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** The markup of an icon whose body is left out. */
const ICON =
    '<svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true"></svg>';

test("render a pagination marking the current page, with labelled steps", () => {
    const container = draw(() => (
        <Pagination>
            <PaginationContent>
                <PaginationItem>
                    <PaginationPrevious href="?page=1" />
                </PaginationItem>
                <PaginationItem>
                    <PaginationLink href="?page=2" isActive>
                        2
                    </PaginationLink>
                </PaginationItem>
                <PaginationItem>
                    <PaginationEllipsis />
                </PaginationItem>
                <PaginationItem>
                    <PaginationNext href="?page=3" />
                </PaginationItem>
            </PaginationContent>
        </Pagination>
    ));
    expect(markup(container)).toBe(
        '<nav aria-label="Pagination" data-slot="pagination"><ul data-slot="pagination-content">' +
            `<li data-slot="pagination-item"><a data-slot="pagination-previous" aria-label="Go to the previous page" href="?page=1">${ICON}<span>Previous</span></a></li>` +
            '<li data-slot="pagination-item"><a aria-current="page" data-slot="pagination-link" data-active="true" href="?page=2">2</a></li>' +
            `<li data-slot="pagination-item"><span data-slot="pagination-ellipsis">${ICON}<span>More pages</span></span></li>` +
            `<li data-slot="pagination-item"><a data-slot="pagination-next" aria-label="Go to the next page" href="?page=3"><span>Next</span>${ICON}</a></li></ul></nav>`,
    );
});
