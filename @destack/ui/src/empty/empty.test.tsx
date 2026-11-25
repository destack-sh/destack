import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import {
    Empty,
    EmptyContent,
    EmptyDescription,
    EmptyHeader,
    EmptyMedia,
    EmptyTitle,
} from "./index.ts";

test("render what is empty with its picture, its words and what to do", () => {
    const { container } = render(() => (
        <Empty>
            <EmptyHeader>
                <EmptyMedia variant="icon">!</EmptyMedia>
                <EmptyTitle>No files</EmptyTitle>
                <EmptyDescription>Open one.</EmptyDescription>
            </EmptyHeader>
            <EmptyContent render={(part) => <section {...part} />}>Open</EmptyContent>
        </Empty>
    ));

    expect(markup(container)).toBe(
        '<div data-slot="empty"><div data-slot="empty-header">' +
            '<div data-slot="empty-media" data-variant="icon">!</div>' +
            '<div data-slot="empty-title">No files</div>' +
            '<div data-slot="empty-description">Open one.</div></div>' +
            '<section data-slot="empty-content">Open</section></div>',
    );
});
