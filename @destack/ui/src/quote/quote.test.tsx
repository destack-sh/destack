import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { Quote, QuoteContent, QuoteDismiss, QuoteIcon } from "./index.ts";

test("render a quote that leads to what it quotes, and one to take back", () => {
    const { container } = render(() => (
        <>
            <Quote render={(attributes) => <a {...attributes} href="#noon" />}>
                <QuoteIcon>*</QuoteIcon>
                <QuoteContent>By noon</QuoteContent>
            </Quote>
            <Quote>
                <QuoteContent>By noon</QuoteContent>
                <QuoteDismiss>x</QuoteDismiss>
            </Quote>
        </>
    ));

    expect(markup(container)).toBe(
        [
            '<a data-slot="quote" href="#noon"><span data-slot="quote-icon" aria-hidden="true">*</span>',
            '<div data-slot="quote-content">By noon</div></a>',
            '<div data-slot="quote"><div data-slot="quote-content">By noon</div>',
            '<button data-variant="ghost" data-size="sm" type="button" data-slot="quote-dismiss" aria-label="Remove quote">x</button></div>',
        ].join(""),
    );
});
