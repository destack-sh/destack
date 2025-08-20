// TODO: define AST in .ds

use std::collections::HashMap;

/// A Span in a SourceFile.
#[derive(Debug, Clone)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: u32,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct SourceMap {
    pub source_files: HashMap<u32, SourceFile>,
}

#[derive(Debug, Clone)]
pub enum Visibility {
    Inherited,
    Module,
    Super,
}

#[derive(Debug, Clone)]
pub struct ModuleDefinition {
    pub id: u32,
    pub span: Span,
    pub visibility: Visibility,
    pub name: String,
    pub items: Vec<ItemDefinition>,
}

#[derive(Debug, Clone)]
pub struct ItemDefinition {
    pub id: u32,
    pub span: Span,
    pub visibility: Visibility,
    pub name: String,
    pub items: Vec<ItemDefinition>,
}
