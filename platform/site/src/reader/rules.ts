/** Filter the complete server-rendered rule directory in place. */
export function enhanceRuleCatalog(body: HTMLElement): () => void {
    // find the catalog, or skip documents without one
    const catalog = body.querySelector<HTMLElement>(".lint-catalog");
    if (!catalog) {
        return () => {};
    }

    // leave the full reference accessible without JavaScript
    const controls = catalog.querySelector<HTMLElement>(".lint-controls")!;
    const search = controls.querySelector<HTMLInputElement>("input")!;
    const filters = [...controls.querySelectorAll<HTMLSelectElement>("select")];

    // find the rows and the toolbar, and enable the controls
    const rows = [...catalog.querySelectorAll<HTMLElement>("[data-rule]")];
    const count = catalog.querySelector<HTMLElement>(".lint-count")!;
    const empty = catalog.querySelector<HTMLElement>(".lint-empty")!;
    const reset = catalog.querySelector<HTMLButtonElement>("[data-clear]")!;
    const toolbar = catalog.querySelector<HTMLElement>(".lint-toolbar")!;
    toolbar.hidden = false;
    for (const filter of filters) {
        filter.disabled = false;
    }
    reset.hidden = false;

    // combine words and facets while preserving unrelated URL state
    const update = (persist = true) => {
        // hide each row that misses a term or a facet
        const terms = search.value.toLowerCase().trim().split(/\s+/).filter(Boolean);
        let visible = 0;
        for (const row of rows) {
            row.hidden =
                !terms.every((term) => row.dataset.search!.includes(term)) ||
                !filters.every(
                    (filter) => !filter.value || row.dataset[filter.name] === filter.value,
                );
            if (!row.hidden) {
                visible++;
            }
        }
        for (const filter of filters) {
            filter.dataset.active = String(Boolean(filter.value));
        }
        count.textContent =
            visible === rows.length ? `${visible} rules` : `${visible} of ${rows.length} rules`;
        empty.hidden = visible !== 0;
        reset.disabled = !search.value && filters.every((filter) => !filter.value);
        if (persist) {
            const url = new URL(location.href);
            for (const control of [search, ...filters]) {
                if (control.value) {
                    url.searchParams.set(control.name, control.value);
                } else {
                    url.searchParams.delete(control.name);
                }
            }
            history.replaceState(history.state, "", url);
        }
    };
    const restore = () => {
        // restore the search and filters from the URL
        const parameters = new URLSearchParams(location.search);
        search.value = parameters.get("q") ?? "";
        for (const filter of filters) {
            filter.value = parameters.get(filter.name) ?? "";
        }
        update(false);
    };
    const input = () => update();
    const clear = () => {
        // clear the search and filters
        search.value = "";
        for (const filter of filters) {
            filter.value = "";
        }
        update();
        search.focus();
    };
    restore();
    controls.addEventListener("input", input);
    reset.addEventListener("click", clear);
    window.addEventListener("popstate", restore);

    return () => {
        // release handlers when leaving the document
        controls.removeEventListener("input", input);
        reset.removeEventListener("click", clear);
        window.removeEventListener("popstate", restore);
    };
}
