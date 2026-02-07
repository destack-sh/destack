use destack_compiler_macros::DefineTask;
use destack_linter::{LintLevel, LintRunner};
use destack_source::{
    ModuleId, ModuleStamp, ModuleVersion, PackageId, PackageStamp, ProfileStamp, ProfileVersion,
};
use destack_workspace::{LintPreset, ProfileId};
use std::sync::LazyLock;

use crate::timing::tags;
use crate::{Compiler, LintError, LintResult, TaskDependencyError, TaskResultCollector};

/// Reuse immutable lint runners by preset to avoid per-module rule allocation.
fn cached_runner_for_preset(preset: LintPreset) -> &'static LintRunner {
    static NONE: LazyLock<LintRunner> =
        LazyLock::new(|| LintRunner::from_preset(LintPreset::None).with_fixes(false));
    static RECOMMENDED: LazyLock<LintRunner> =
        LazyLock::new(|| LintRunner::from_preset(LintPreset::Recommended).with_fixes(false));
    static ALL: LazyLock<LintRunner> =
        LazyLock::new(|| LintRunner::from_preset(LintPreset::All).with_fixes(false));

    match preset {
        LintPreset::None => &NONE,
        LintPreset::Recommended => &RECOMMENDED,
        LintPreset::All => &ALL,
    }
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
        let runner = cached_runner_for_preset(options.preset);

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
        for diagnostic in ast_diagnostics.into_iter().chain(dir_diagnostics) {
            self.program
                .diagnostics
                .insert(diagnostic.into_diagnostic());
        }

        Ok(())
    }

    /// Lint a package.
    fn lint_package(&self, package_id: PackageId) -> LintResult<()> {
        // collect all modules in the package
        let modules = self
            .program
            .modules
            .iter()
            .filter(|module| module.read().package_id == package_id);

        // lint each module
        let mut collector = TaskResultCollector::new();
        for module in modules {
            let module_id = module.read().id;
            let profile = self.program.default_profile_id_for_module(module_id);
            let result = self.require_lint_module(module_id, profile);
            collector.try_collect(result);
        }

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(LintError::Yield { dependency });
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
        test.check_has_diagnostic("LU010");
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
        test.check_has_diagnostic("LU014");
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
        test.check_has_diagnostic("LC014");
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
        test.check_has_diagnostic("LC038");
    }
}
