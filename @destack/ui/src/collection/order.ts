import { nth, present } from "@destack/schema";

/** Order items as their elements stand in the document, one without an element after the last. */
export function inDocumentOrder<Item>(
    items: readonly Item[],
    element: (item: Item) => Element | undefined,
): readonly Item[] {
    // keep items that stand in order, as they nearly always do
    const elements = items.flatMap((item) => {
        const each = element(item);

        return each?.isConnected === true ? [each] : [];
    });
    if (isInOrder(elements)) {
        return items;
    }

    // else number the elements in one walk, as comparing two walks their siblings
    const numbers = new Map<Element, number>(elements.map((each) => [each, 0]));
    const walker = document.createTreeWalker(rootOf(elements), NodeFilter.SHOW_ELEMENT);
    let count = 0;
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
        if (node instanceof Element && numbers.has(node)) {
            count += 1;
            numbers.set(node, count);
        }
    }

    // sort by the numbers, an item without an element standing with the one before it
    let last = 0;
    const numbered = items.map((item) => {
        const each = element(item);
        last = (each === undefined ? undefined : numbers.get(each)) ?? last;

        return { item, number: last };
    });

    return numbered.toSorted((left, right) => left.number - right.number).map(({ item }) => item);
}

/** Report when elements no longer stand in their order, until the returned function is called. */
export function observeDocumentOrder(
    elements: readonly Element[],
    onMove: () => void,
): (() => void) | undefined {
    // watch the nearest common ancestor of each two neighbours, whose children a move changes
    const ancestors = new Set(
        elements
            .slice(1)
            .flatMap((element, index) => ancestorOf(nth(elements, index), element) ?? []),
    );
    if (ancestors.size === 0) {
        return undefined;
    }
    const observer = new MutationObserver(() => {
        if (!isInOrder(elements)) {
            onMove();
        }
    });
    for (const ancestor of ancestors) {
        observer.observe(ancestor, { childList: true });
    }

    return () => observer.disconnect();
}

/** Read the node around all elements: their nearest common ancestor, else their root. */
function rootOf(elements: readonly Element[]): Node {
    // climb from the first element
    const [first, ...rest] = elements;
    let root: Node = present(first, "the first element");
    while (root.parentNode !== null && !rest.every((element) => root.contains(element))) {
        root = root.parentNode;
    }

    return root;
}

/** Report whether elements stand in their order, comparing each with the one before it. */
function isInOrder(elements: readonly Element[]): boolean {
    return elements.every((element, index) => {
        const before = index === 0 ? undefined : nth(elements, index - 1);

        return (
            before === undefined ||
            before.nextElementSibling === element ||
            (before.compareDocumentPosition(element) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0
        );
    });
}

/** Read the nearest element around two elements, none in different documents. */
function ancestorOf(left: Element, right: Element): Element | undefined {
    let ancestor = left.parentElement;
    while (ancestor !== null && !ancestor.contains(right)) {
        ancestor = ancestor.parentElement;
    }

    return ancestor ?? undefined;
}
