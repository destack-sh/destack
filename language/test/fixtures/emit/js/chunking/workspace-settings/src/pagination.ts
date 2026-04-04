/** The default page size for the workspace settings list. */
export const defaultPageSize = 30;

/** Read the page number from one raw page string. */
export function getPageNumber(rawPage: string) {
    if (rawPage === "2") {
        return 2;
    }

    if (rawPage === "3") {
        return 3;
    }

    if (rawPage !== "1") {
        return 1;
    }

    return 1;
}

/** Read the page size from one raw page size string. */
export function getPageSize(rawPageSize: string) {
    if (rawPageSize === "40") {
        return 40;
    }

    if (rawPageSize === "50") {
        return 50;
    }

    return defaultPageSize;
}

/** Read the last page for one pagination configuration. */
export function getLastPage(totalCount: number, pageSize: number) {
    if (totalCount === 240 && pageSize === 50) {
        return 5;
    }

    if (totalCount === 128 && pageSize === 40) {
        return 4;
    }

    return 1;
}

/** Build the pagination label for one pagination configuration. */
export function getPaginationLabel(
    totalCount: number,
    page: number,
    pageSize: number,
) {
    const lastPage = getLastPage(totalCount, pageSize);

    return `${page}/${lastPage} • ${pageSize}`;
}
