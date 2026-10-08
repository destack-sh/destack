import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import {
    Pagination,
    PaginationContent,
    PaginationEllipsis,
    PaginationItem,
    PaginationLink,
    PaginationNext,
    PaginationPages,
    PaginationPrevious,
    paginationEntries,
} from "./index.ts";
import { markup, render } from "@destack/view/test";

/** The markup of an icon whose body is left out. */
const ICON =
    '<svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true" data-slot="icon"></svg>';

test("render a pagination marking the current page, with labelled steps", () => {
    const { container } = render(() => (
        <Pagination>
            <PaginationContent>
                <PaginationItem>
                    <PaginationPrevious href="?page=1" />
                </PaginationItem>
                <PaginationItem>
                    <PaginationLink href="?page=2" active>
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

test("list the pages at either end and around the current one, with gaps between", () => {
    expect([
        paginationEntries(1, 5),
        paginationEntries(6, 20),
        paginationEntries(20, 20),
        paginationEntries(10, 20, 2, 2),
    ]).toEqual([
        [1, 2, 3, 4, 5],
        [1, "ellipsis", 5, 6, 7, "ellipsis", 20],
        [1, "ellipsis", 16, 17, 18, 19, 20],
        [1, 2, "ellipsis", 8, 9, 10, 11, 12, "ellipsis", 19, 20],
    ]);
});

test("page in place when the owner handles the change, marking the current page", () => {
    const changes: number[] = [];
    const { container } = render(() => (
        <Pagination>
            <PaginationPages
                count={20}
                defaultPage={6}
                href={(page) => `?page=${page}`}
                onPageChange={(page) => changes.push(page)}
            />
        </Pagination>
    ));
    container.querySelector<HTMLElement>("[data-slot=pagination-next]")?.click();
    flush();
    expect([changes, container.querySelector("[aria-current=page]")?.textContent]).toEqual([
        [7],
        "7",
    ]);
});
