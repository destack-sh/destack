use std::fmt;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::{DiagnosticSeverity, FileId, ModuleId, Span};

use crate::DiagnosticAnchor;

/// Checked diagnostic controls for one module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DiagnosticControlTable {
    /// The controlled module.
    pub module: ModuleId,
    /// The physical files belonging to the module.
    pub files: Vec<FileId>,
    /// The checked controls in stable source order within each scope.
    pub controls: Vec<DiagnosticControl>,
}

/// One checked diagnostic control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DiagnosticControl {
    /// The diagnostic id source span.
    pub source: Span,
    /// The controlled lexical scope.
    pub scope: DiagnosticControlScope,
    /// The selected diagnostic level.
    pub level: DiagnosticControlLevel,
    /// The canonical diagnostic id.
    pub diagnostic: StringId,
    /// The optional authored reason.
    pub reason: Option<StringId>,
}

/// The lexical scope controlled by one diagnostic decorator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DiagnosticControlScope {
    /// The complete source module.
    Module,
    /// One lexical source subtree.
    Span(Span),
}

/// The effect of one checked diagnostic control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DiagnosticControlLevel {
    /// Suppress matching diagnostics.
    Allow,
    /// Report matching diagnostics as warnings.
    Warn,
    /// Report matching diagnostics as errors.
    Deny,
    /// Report matching diagnostics as errors and reject inner overrides.
    Forbid,
    /// Suppress one matching diagnostic and require that it occurs.
    Expect,
}

/// Checked diagnostic controls indexed by their source owners.
#[derive(Debug)]
pub struct DiagnosticControlIndex<'a> {
    /// The checked control tables.
    tables: Box<[&'a DiagnosticControlTable]>,
    /// Control table indices by module.
    modules: FxHashMap<ModuleId, usize>,
    /// Control table indices by physical file.
    files: FxHashMap<FileId, usize>,
}

/// An invalid diagnostic control index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticControlIndexError {
    /// One module has multiple checked control tables.
    DuplicateModule(ModuleId),
    /// One physical file belongs to multiple checked modules.
    DuplicateFile(FileId),
}

impl<'a> TryFrom<&'a str> for DiagnosticControlLevel {
    type Error = &'a str;

    /// Convert one standard name into a diagnostic control level.
    fn try_from(name: &'a str) -> Result<Self, Self::Error> {
        match name {
            "allow" => Ok(Self::Allow),
            "warn" => Ok(Self::Warn),
            "deny" => Ok(Self::Deny),
            "forbid" => Ok(Self::Forbid),
            "expect" => Ok(Self::Expect),
            _ => Err(name),
        }
    }
}

impl TryFrom<dir::LanguageItem> for DiagnosticControlLevel {
    type Error = dir::LanguageItem;

    /// Convert one diagnostic decorator language item into its control level.
    fn try_from(item: dir::LanguageItem) -> Result<Self, Self::Error> {
        match item {
            dir::LanguageItem::Allow => Ok(Self::Allow),
            dir::LanguageItem::Warn => Ok(Self::Warn),
            dir::LanguageItem::Deny => Ok(Self::Deny),
            dir::LanguageItem::Forbid => Ok(Self::Forbid),
            dir::LanguageItem::Expect => Ok(Self::Expect),
            item => Err(item),
        }
    }
}

impl DiagnosticControlLevel {
    /// Return the diagnostic severity selected by this level.
    pub fn severity(self) -> Option<DiagnosticSeverity> {
        match self {
            Self::Allow | Self::Expect => None,
            Self::Warn => Some(DiagnosticSeverity::Warning),
            Self::Deny | Self::Forbid => Some(DiagnosticSeverity::Error),
        }
    }
}

impl DiagnosticControlTable {
    /// Create an empty checked control table for one module.
    pub fn new(module: ModuleId, files: Vec<FileId>) -> Self {
        Self {
            module,
            files,
            controls: Vec::new(),
        }
    }

    /// Append one checked control.
    pub fn push(&mut self, control: DiagnosticControl) {
        self.controls.push(control);
    }

    /// Return the number of checked controls.
    pub fn len(&self) -> usize {
        self.controls.len()
    }

    /// Return whether there are no checked controls.
    pub fn is_empty(&self) -> bool {
        self.controls.is_empty()
    }

    /// Return whether one control activates a diagnostic.
    pub fn activates(&self, diagnostic: StringId) -> bool {
        self.controls.iter().any(|control| {
            control.diagnostic == diagnostic
                && !matches!(control.level, DiagnosticControlLevel::Allow)
        })
    }

    /// Return the enclosing forbid that rejects one control override.
    pub fn enclosing_forbid(&self, control: &DiagnosticControl) -> Option<&DiagnosticControl> {
        self.controls.iter().rev().find(|previous| {
            matches!(previous.level, DiagnosticControlLevel::Forbid)
                && !matches!(control.level, DiagnosticControlLevel::Forbid)
                && previous.diagnostic == control.diagnostic
                && previous.scope.contains(control.scope)
        })
    }

    /// Return the effective control and index for one diagnostic.
    fn effective(
        &self,
        diagnostic: StringId,
        anchor: &DiagnosticAnchor,
    ) -> Option<(usize, &DiagnosticControl)> {
        // inspect every checked control in stable order
        let mut effective: Option<(usize, &DiagnosticControl)> = None;
        for (index, control) in self.controls.iter().enumerate() {
            // ignore controls that do not select or contain this diagnostic
            if control.diagnostic != diagnostic || !control.scope.contains_anchor(anchor) {
                continue;
            }

            // preserve a matching forbid against every inner control
            if matches!(control.level, DiagnosticControlLevel::Forbid) {
                return Some((index, control));
            }

            // prefer inner scopes and later controls on the same scope
            let should_replace = match effective {
                Some((_, previous)) => control.scope.overrides(previous.scope),
                None => true,
            };
            if should_replace {
                effective = Some((index, control));
            }
        }

        effective
    }

    /// Iterate expectations for one diagnostic.
    pub fn expectations<'a>(
        &'a self,
        diagnostic: StringId,
    ) -> impl Iterator<Item = (usize, &'a DiagnosticControl)> + 'a {
        self.controls
            .iter()
            .enumerate()
            .filter(move |(_, control)| {
                control.diagnostic == diagnostic
                    && matches!(control.level, DiagnosticControlLevel::Expect)
            })
    }
}

impl<'a> DiagnosticControlIndex<'a> {
    /// Index checked control tables by their source owners.
    pub fn new(
        tables: impl IntoIterator<Item = &'a DiagnosticControlTable>,
    ) -> Result<Self, DiagnosticControlIndexError> {
        let tables = tables.into_iter().collect::<Vec<_>>();
        let mut modules = FxHashMap::default();
        let mut files = FxHashMap::default();

        // index exact module and file ownership
        for (index, table) in tables.iter().enumerate() {
            if modules.insert(table.module, index).is_some() {
                return Err(DiagnosticControlIndexError::DuplicateModule(table.module));
            }
            for file in &table.files {
                if files.insert(*file, index).is_some() {
                    return Err(DiagnosticControlIndexError::DuplicateFile(*file));
                }
            }
        }

        Ok(Self {
            tables: tables.into_boxed_slice(),
            modules,
            files,
        })
    }

    /// Iterate indexed control tables in input order.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &DiagnosticControlTable)> {
        self.tables.iter().copied().enumerate()
    }

    /// Return the effective control and table indices for one diagnostic.
    pub fn effective(
        &self,
        diagnostic: StringId,
        anchor: &DiagnosticAnchor,
    ) -> Option<(usize, usize, &DiagnosticControl)> {
        let (table_index, table) = self.table(anchor)?;
        let (control_index, control) = table.effective(diagnostic, anchor)?;

        Some((table_index, control_index, control))
    }

    /// Return the control table that owns one diagnostic anchor.
    fn table(&self, anchor: &DiagnosticAnchor) -> Option<(usize, &DiagnosticControlTable)> {
        let index = match anchor {
            DiagnosticAnchor::Span(span) => self.files.get(&span.file),
            DiagnosticAnchor::File(file) => self.files.get(file),
            DiagnosticAnchor::Module(module) => self.modules.get(module),
            DiagnosticAnchor::Package(_) => None,
        }?;
        let table = self.tables[*index];

        Some((*index, table))
    }
}

impl fmt::Display for DiagnosticControlIndexError {
    /// Format the violated source ownership invariant.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateModule(module) => {
                write!(
                    formatter,
                    "module {module:?} has multiple diagnostic control tables"
                )
            }
            Self::DuplicateFile(file) => {
                write!(
                    formatter,
                    "source file {file} belongs to multiple checked modules"
                )
            }
        }
    }
}

impl std::error::Error for DiagnosticControlIndexError {}

impl DiagnosticControlScope {
    /// Return whether this scope contains another lexical scope.
    pub fn contains(self, other: Self) -> bool {
        match (self, other) {
            (Self::Module, _) => true,
            (Self::Span(_), Self::Module) => false,
            (Self::Span(scope), Self::Span(other)) => scope.contains_span(other),
        }
    }

    /// Return whether this scope contains one table-owned diagnostic anchor.
    fn contains_anchor(self, anchor: &DiagnosticAnchor) -> bool {
        match self {
            Self::Module => true,
            Self::Span(scope) => match anchor {
                DiagnosticAnchor::Span(span) => scope.contains_span(*span),
                DiagnosticAnchor::File(_)
                | DiagnosticAnchor::Module(_)
                | DiagnosticAnchor::Package(_) => false,
            },
        }
    }

    /// Return whether this scope overrides one previously selected scope.
    fn overrides(self, previous: Self) -> bool {
        previous.contains(self)
    }
}
