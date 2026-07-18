use destack_core::StringId;
use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{DiagnosticSeverity, FileId, ModuleId, Span};
use serde::{Deserialize, Serialize};

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
    /// The selector source span.
    pub source: Span,
    /// The controlled lexical scope.
    pub scope: DiagnosticControlScope,
    /// The selected diagnostic level.
    pub level: DiagnosticControlLevel,
    /// The stable lint id or diagnostic code selector.
    pub selector: StringId,
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

    /// Return whether one control activates an otherwise disabled selector.
    pub fn enables(&self, selectors: &[StringId]) -> bool {
        self.controls.iter().any(|control| {
            control.selects(selectors) && !matches!(control.level, DiagnosticControlLevel::Allow)
        })
    }

    /// Return the enclosing forbid that rejects one control override.
    pub fn enclosing_forbid(&self, control: &DiagnosticControl) -> Option<&DiagnosticControl> {
        self.controls.iter().rev().find(|previous| {
            matches!(previous.level, DiagnosticControlLevel::Forbid)
                && !matches!(control.level, DiagnosticControlLevel::Forbid)
                && previous.selector == control.selector
                && previous.scope.contains(control.scope)
        })
    }

    /// Return the effective control and index for one diagnostic.
    pub fn effective(
        &self,
        selectors: &[StringId],
        anchor: &DiagnosticAnchor,
    ) -> Option<(usize, &DiagnosticControl)> {
        // reject anchors owned by another module
        if !self.owns(anchor) {
            return None;
        }

        // inspect every checked control in stable order
        let mut effective: Option<(usize, &DiagnosticControl)> = None;
        for (index, control) in self.controls.iter().enumerate() {
            // ignore controls that do not select or contain this diagnostic
            if !control.selects(selectors) || !control.scope.contains_anchor(anchor) {
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

    /// Iterate expectations for one selector.
    pub fn expectations<'a>(
        &'a self,
        selectors: &'a [StringId],
    ) -> impl Iterator<Item = (usize, &'a DiagnosticControl)> + 'a {
        self.controls
            .iter()
            .enumerate()
            .filter(move |(_, control)| {
                control.selects(selectors)
                    && matches!(control.level, DiagnosticControlLevel::Expect)
            })
    }

    /// Return whether one diagnostic anchor belongs to this module.
    fn owns(&self, anchor: &DiagnosticAnchor) -> bool {
        match anchor {
            DiagnosticAnchor::Span(span) => self.files.contains(&span.file),
            DiagnosticAnchor::File(file) => self.files.contains(file),
            DiagnosticAnchor::Module(module) => *module == self.module,
            DiagnosticAnchor::Package(_) => false,
        }
    }
}

impl DiagnosticControl {
    /// Return whether this control selects one accepted diagnostic identity.
    fn selects(&self, selectors: &[StringId]) -> bool {
        selectors.contains(&self.selector)
    }
}

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
