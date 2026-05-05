mod generated {
    include!(concat!(env!("OUT_DIR"), "/library_sources.rs"));
}

use super::source::LibraryPackage;

use generated::{CORE_DECLARED_SYMBOLS, CORE_SOURCES, PLATFORM_SOURCES, STANDARD_SOURCES};

pub const LIB_CORE: LibraryPackage = LibraryPackage::language("core", CORE_SOURCES, &[])
    .with_declared_symbols(CORE_DECLARED_SYMBOLS);

pub const LIB_PLATFORM: LibraryPackage =
    LibraryPackage::library("platform", PLATFORM_SOURCES, &["core"]).explicit();

pub const LIB_DESTACK: LibraryPackage =
    LibraryPackage::library("destack", STANDARD_SOURCES, &["core", "platform"]).explicit();

/// Language libraries known to the compiler.
pub const LANGUAGE_LIBS: &[LibraryPackage] = &[LIB_CORE];

/// Standard library packages known to the compiler.
pub const LIBRARY_PACKAGES: &[LibraryPackage] = &[LIB_DESTACK, LIB_PLATFORM];
