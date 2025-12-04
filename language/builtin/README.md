# Destack Built-ins

Language built-in definitions written in Destack. Shipped with the language toolchain.
This package is `"private": true` - it's bundled in the compiler (no import needed), not published to npm separately. 
Built-ins define the language-level primitives. Optional libraries extend them (like `@destack-sh/schema` extends `Type` for general schemas).

## Structure

```
src/
  type.ds         # typeOf, Type<T>, Field, Method, reflection primitives
```