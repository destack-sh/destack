use destack_compiler_macros::DefineTask;
use destack_linter::{LintDiagnostic, LintLevel, LintRunner};
use destack_source::{
    FileId, ModuleId, ModuleStamp, ModuleVersion, PackageId, PackageStamp, ProfileStamp,
    ProfileVersion,
};
use destack_workspace::{LintPreset, LinterOptions, ProfileId};
use std::collections::HashSet;
use std::sync::LazyLock;

use crate::timing::tags;
use crate::{Compiler, LintError, LintResult, TaskDependencyError, TaskResultCollector};

/// Reuse immutable lint runners by preset to avoid per-module rule allocation.
fn cached_runner_for_options(options: &LinterOptions) -> &'static LintRunner {
    static NONE: LazyLock<LintRunner> =
        LazyLock::new(|| LintRunner::from_preset(LintPreset::None).with_fixes(false));
    static RECOMMENDED: LazyLock<LintRunner> =
        LazyLock::new(|| LintRunner::from_preset(LintPreset::Recommended).with_fixes(false));
    static STRICT: LazyLock<LintRunner> =
        LazyLock::new(|| LintRunner::from_preset(LintPreset::Strict).with_fixes(false));
    static ALL: LazyLock<LintRunner> =
        LazyLock::new(|| LintRunner::from_preset(LintPreset::All).with_fixes(false));

    // explicit category or rule overrides can enable any rule
    if !options.categories.is_empty() || !options.overrides.is_empty() {
        return &ALL;
    }

    match options.preset {
        LintPreset::None => &NONE,
        LintPreset::Recommended => &RECOMMENDED,
        LintPreset::Strict => &STRICT,
        LintPreset::All => &ALL,
    }
}

/// Key used to deduplicate lint diagnostics.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LintDiagnosticKey {
    /// The diagnostic code.
    code: String,
    /// The file id.
    file_id: FileId,
    /// Primary span start offset.
    start: u32,
    /// Primary span end offset.
    end: u32,
    /// Diagnostic message.
    message: String,
}

/// Task to lint something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Lint)]
pub enum LintTask {
    /// Lint a module.
    #[task(code = 1, trace = "module={module} profile={profile}")]
    LintModule {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Lint a package
    #[task(code = 2, trace = "package={package}")]
    LintPackage { package: PackageStamp },
}

impl Compiler {
    /// Convert one source diagnostic to a lint dedupe key.
    fn lint_key_from_source_diagnostic(
        diagnostic: &destack_source::Diagnostic,
    ) -> LintDiagnosticKey {
        LintDiagnosticKey {
            code: diagnostic.code.clone(),
            file_id: diagnostic.file_id,
            start: diagnostic.primary_span.span.start,
            end: diagnostic.primary_span.span.end,
            message: diagnostic.message.clone(),
        }
    }

    /// Convert one lint diagnostic to a dedupe key.
    fn lint_key_from_lint_diagnostic(diagnostic: &LintDiagnostic) -> LintDiagnosticKey {
        LintDiagnosticKey {
            code: diagnostic.code.to_string(),
            file_id: diagnostic.file_id,
            start: diagnostic.span.start,
            end: diagnostic.span.end,
            message: diagnostic.message.clone(),
        }
    }

    /// Insert lint diagnostics while deduplicating against existing diagnostics.
    fn insert_lint_diagnostics(&self, diagnostics: impl IntoIterator<Item = LintDiagnostic>) {
        // seed dedupe state from already emitted diagnostics
        let mut seen = HashSet::new();
        for diagnostic in self.program.diagnostics.iter() {
            if diagnostic.code.starts_with('L') {
                seen.insert(Self::lint_key_from_source_diagnostic(&diagnostic));
            }
        }

        // insert only fresh lint diagnostics
        for diagnostic in diagnostics {
            let key = Self::lint_key_from_lint_diagnostic(&diagnostic);
            if seen.insert(key) {
                self.program
                    .diagnostics
                    .insert(diagnostic.into_diagnostic());
            }
        }
    }

    /// Process a lint task.
    pub fn process_lint(&self, task: LintTask) -> LintResult<()> {
        match task {
            LintTask::LintModule { module, profile } => {
                self.ensure_module_profile_matches::<LintError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                let _timing = self.timing_scope(tags::LINT_MODULE);
                self.lint_module(module.id, profile.id, module.version, profile.version)?;
            }
            LintTask::LintPackage { package } => {
                self.ensure_package_version_matches::<LintError>(package.id, package.version)?;
                let _timing = self.timing_scope(tags::LINT_PACKAGE);
                self.lint_package(package.id)?;
            }
        };
        Ok(())
    }

    /// Ensure a module has been analyzed.
    pub fn require_lint_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(LintTask::LintModule {
            module: self.module_stamp(module_id),
            profile: self.profile_stamp(profile),
        })
    }

    /// Ensure a package has been linted.
    pub fn require_lint_package(&self, package_id: PackageId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(LintTask::LintPackage {
            package: self.package_stamp(package_id),
        })
    }

    /// Lint a module.
    fn lint_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> LintResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<LintError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;

        self.require_analyze_module(module_id, profile)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }
        self.stats.record_lint();

        let options = self.program.get_linter_options(module_id);
        if !options.enabled {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);
        let runner = cached_runner_for_options(&options);

        // run at each IR level
        let ast_diagnostics = runner.lint_module(
            self.program.clone(),
            module.clone(),
            profile,
            &options,
            LintLevel::Ast,
        );
        let dir_diagnostics = runner.lint_module(
            self.program.clone(),
            module.clone(),
            profile,
            &options,
            LintLevel::Dir,
        );

        // TODO #Incomplete: run comptime/user-defined lints?

        // collect diagnostics
        self.insert_lint_diagnostics(ast_diagnostics.into_iter().chain(dir_diagnostics));

        Ok(())
    }

    /// Lint a package.
    fn lint_package(&self, package_id: PackageId) -> LintResult<()> {
        // collect all modules in the package
        let mut module_ids: Vec<_> = self
            .program
            .modules
            .iter()
            .filter(|module| module.read().package_id == package_id)
            .map(|module| module.read().id)
            .collect();
        module_ids.sort_unstable();

        // lint each module
        let mut collector = TaskResultCollector::new();
        for module_id in &module_ids {
            let profile = self.program.default_profile_id_for_module(*module_id);
            let result = self.require_lint_module(*module_id, profile);
            collector.try_collect(result);
        }

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(LintError::Yield { dependency });
        }

        // run program scoped AST rules once per package lint pass
        let Some(options_module_id) = module_ids
            .iter()
            .copied()
            .find(|module_id| self.is_code_module(*module_id))
        else {
            return Ok(());
        };
        let options = self.program.get_linter_options(options_module_id);
        if !options.enabled {
            return Ok(());
        }
        let runner = cached_runner_for_options(&options);
        let ast_diagnostics = runner.lint_program_ast(self.program.clone(), &options);
        self.insert_lint_diagnostics(ast_diagnostics);

        // run program scoped DIR rules once per profile used by this package
        let mut profiles = HashSet::new();
        for module_id in &module_ids {
            let profile = self.program.default_profile_id_for_module(*module_id);
            profiles.insert(profile);
        }
        for profile in profiles {
            let dir_diagnostics = runner.lint_program_dir(self.program.clone(), profile, &options);
            self.insert_lint_diagnostics(dir_diagnostics);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_lint_module_detects_debugger() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "test.ds",
            r#"
debugger;
"#,
        );
        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LU008");
    }

    #[test]
    fn test_lint_module_detects_empty_block() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "test.ds", r#"
{}
"#,
        );
        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LU012");
    }

    #[test]
    fn test_lint_module_detects_constant_condition() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "test.ds",
            r#"
if (true) { foo(); }
"#,
        );
        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LC009");
    }

    #[test]
    fn test_lint_module_detects_self_compare() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "test.ds",
            r#"
let x = 1;
x == x;
"#,
        );
        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LC025");
    }

    #[test]
    fn test_lint_package_runs_program_ast_lints() {
        let test = TestProgram::memory_sequential();
        let first_module = test.add_module(
            "program_ast/first.ds",
            r#"
function shared(): int32 {
    const first = 1;
    const second = 2;
    const third = first + second;
    return third + 10;
}
"#,
        );
        let _second_module = test.add_module(
            "program_ast/second.ds",
            r#"
function shared(): int32 {
    const first = 1;
    const second = 2;
    const third = first + second;
    return third + 10;
}
"#,
        );
        test.apply_dsconfig(
            first_module,
            r#"
{
  "linter": {
    "rules": {
      "all": true
    }
  }
}
"#,
        );

        let package_id = test.program.modules.get(first_module).read().package_id;
        test.enqueue(super::LintTask::LintPackage {
            package: test.package_stamp(package_id),
        });
        test.compile();
        test.check_has_diagnostic("LX017");
    }

    #[test]
    fn test_lint_module_strict_preset_enables_strict_rules() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "strict_preset/test.ds",
            r#"
function id(value: int32, unused: int32): int32 {
    return value;
}
"#,
        );
        test.apply_dsconfig(
            module,
            r#"
{
  "linter": {
    "rules": {
      "preset": "strict"
    }
  }
}
"#,
        );

        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LC036");
    }

    #[test]
    fn test_lint_module_rule_override_enables_non_preset_rule() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "override_rule/test.ds",
            r#"
var value = 1;
"#,
        );
        test.apply_dsconfig(
            module,
            r#"
{
  "linter": {
    "rules": {
      "preset": "recommended",
      "no-var": "error"
    }
  }
}
"#,
        );

        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LY026");
    }
}
