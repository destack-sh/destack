import type { JSX } from "@solidjs/web";
import {
    Pagination,
    PaginationContent,
    PaginationEllipsis,
    PaginationItem,
    PaginationLink,
    PaginationNext,
    PaginationPrevious,
} from "./pagination.tsx";

/** Show the pages of a long note list, on its second page. */
export function PaginationExample(): JSX.Element {
    return (
        <Pagination>
            <PaginationContent>
                <PaginationItem>
                    <PaginationPrevious href="?page=1" />
                </PaginationItem>
                <PaginationItem>
                    <PaginationLink href="?page=1">1</PaginationLink>
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
    );
}
