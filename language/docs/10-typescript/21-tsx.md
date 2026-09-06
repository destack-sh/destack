---
title: TSX
description: Tag based trees are pretty useful and broadly applicable.
---

# TSX

- tag based trees are pretty useful and broadly applicable
- full TSX support (no arbitrary XMLNS namespaces though)
- there are other ways of doing UI, but this is a pretty good one, and it's *very* familiar
- trees, generalised tree litearls,
- lowercase tree builders, ..?
- support both "Elements" (like `<Panel />`) and "fragments" (`<div />`)
- (unfortunately this also means keeping TS ambiguity around..)

- contextual TreeBuilder interface incl. string tags

- tree literals build through a contextual `TreeBuilder`
otherwise `compiler.tree` names the default builder as `<specifier>#<export>`
- lowercase tags are keys of the builder's `Tags` row and call its static `element`; fragments call its static `fragment`
- uppercase tags resolve ordinary lexical values: functions receive a props object, classes receive constructor props, and structs receive literal fields
- written and spread attributes merge under TSX rules; every required property must be present and every contributed property must exist on the target row
- children synthesize the `children` property and form a source-ordered tuple
- text remains a string literal and spread children must have statically known tuple length

```tsx:src/view.tsx
export function Greeting({ name }: { name: string }) {
    return <h1>Hello, {name}!</h1>;
}
```
