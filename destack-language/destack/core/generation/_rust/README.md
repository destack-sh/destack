# Rust Generation

Rust is the core of the Destack system, but the "language" itself is defined in Python.
We generate everything from that Python source of truth (destack-language),
 and we follow the same domain/category/module layout everywhere.

To make working with the generated stuff as seamless as possible
 - without losing sight of the code and data that's actually generated - 
 we generate two types of files from the source module tree:
  1) object.rs - Object definition, partials and stubs, all custom stuff.
     This is partially generated and automatically patched on re-gen.
  2) object_gen.rs - generated utilities and such.
     This is fully generated and replaced on every re-gen. Do not edit.

 