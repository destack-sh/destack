import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import {
    Empty,
    EmptyContent,
    EmptyDescription,
    EmptyHeader,
    EmptyMedia,
    EmptyTitle,
} from "./index.ts";

test("render each element of an empty state with its data-slot", () => {
    const container = draw(() => (
        <Empty>
            <EmptyHeader>
                <EmptyMedia variant="icon">N</EmptyMedia>
                <EmptyTitle>No notebooks yet</EmptyTitle>
                <EmptyDescription>Notebooks keep related notes together.</EmptyDescription>
            </EmptyHeader>
            <EmptyContent>Create a notebook</EmptyContent>
        </Empty>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="empty"><div data-slot="empty-header">' +
            '<div data-slot="empty-media" data-variant="icon">N</div>' +
            '<div data-slot="empty-title">No notebooks yet</div>' +
            '<div data-slot="empty-description">Notebooks keep related notes together.</div></div>' +
            '<div data-slot="empty-content">Create a notebook</div></div>',
    );
});
