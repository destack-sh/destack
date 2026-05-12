# Third-Party Licenses

Destack includes vendored code from the following third-party projects.
These components are used under their respective licenses as noted below.

---

## Cranelift

**License:** Apache-2.0 WITH LLVM-exception
**Authors:** The Cranelift Project Developers
**Source:** https://github.com/bytecodealliance/wasmtime
**Location:** `language/compiler/generate/native/cranelift/`
**Modifications:** This copy has been modified for Destack.

---

## regalloc2

**License:** Apache-2.0 WITH LLVM-exception
**Authors:** Chris Fallin, Mozilla SpiderMonkey Developers
**Source:** https://github.com/bytecodealliance/regalloc2
**Location:** `language/compiler/generate/native/regalloc2/`
**Modifications:** This copy has been modified for Destack.

---

## wasmtime-core

**License:** Apache-2.0 WITH LLVM-exception
**Authors:** The Wasmtime Project Developers
**Source:** https://github.com/bytecodealliance/wasmtime
**Location:** `language/compiler/generate/native/wasmtime-core/`
**Modifications:** This copy has been modified for Destack.

---

## Pulley

**License:** Apache-2.0 WITH LLVM-exception
**Authors:** The Pulley Project Developers
**Source:** https://github.com/bytecodealliance/wasmtime
**Location:** `language/compiler/generate/native/pulley/`
**Modifications:** This copy has been modified for Destack.

---

## License Text

The full text of the Apache License 2.0 with LLVM Exception can be found at:
`language/compiler/generate/native/cranelift/codegen/LICENSE`

### LLVM Exception Summary

The LLVM Exception allows compiled output (binaries produced by Destack) to be
distributed without the attribution requirements of the Apache License. This
means users compiling code with Destack do not need to include these license
notices in their own programs.

The exception text:

> As an exception, if, as a result of your compiling your source code, portions
> of this Software are embedded into an Object form of such source code, you
> may redistribute such embedded portions in such Object form without complying
> with the conditions of Sections 4(a), 4(b) and 4(d) of the License.
