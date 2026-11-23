import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { DataList, DataListItem, DataListLabel, DataListValue } from "./index.ts";

test("list each label and its value as a description list, beside it by default or above it", () => {
    const { container } = render(() => (
        <>
            <DataList>
                <DataListItem>
                    <DataListLabel>Code</DataListLabel>
                    <DataListValue>500</DataListValue>
                </DataListItem>
            </DataList>
            <DataList orientation="vertical">
                <DataListItem>
                    <DataListLabel>Request</DataListLabel>
                    <DataListValue>req_7f3a</DataListValue>
                </DataListItem>
            </DataList>
        </>
    ));

    expect(markup(container)).toBe(
        '<dl data-slot="data-list" data-orientation="horizontal">' +
            '<div data-slot="data-list-item">' +
            '<dt data-slot="data-list-label">Code</dt>' +
            '<dd data-slot="data-list-value">500</dd></div></dl>' +
            '<dl data-slot="data-list" data-orientation="vertical">' +
            '<div data-slot="data-list-item">' +
            '<dt data-slot="data-list-label">Request</dt>' +
            '<dd data-slot="data-list-value">req_7f3a</dd></div></dl>',
    );
});
