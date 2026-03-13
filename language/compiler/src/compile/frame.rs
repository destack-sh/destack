use std::cell::RefCell;
use std::sync::Arc;

use destack_source::ModuleId;
use destack_workspace::{ModuleDir, ModuleDirData, ProfileId};

use crate::analyze::DirReadBoundary;
use crate::{BuildKey, Compiler};

// FUGU #Architecture: remove all "build frame" stuff post refactor
thread_local! {
    /// The current task-local transient build frame
    static CURRENT_BUILD_FRAME: RefCell<BuildFrame> = RefCell::new(BuildFrame::default());
}

/// Return the strongest active transient DIR for one module and profile.
///
/// FUGU #Architecture: slice 3 still lets diagnostic formatting consult the
/// current task-local DIR builder until narrower artifact-backed builders
/// replace whole transient DIR reconstruction.
pub(crate) fn current_active_dir(
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Option<Arc<ModuleDir>> {
    CURRENT_BUILD_FRAME.with(|frame| {
        let frame = frame.borrow();
        let active = frame
            .active_dirs
            .iter()
            .rev()
            .find(|entry| entry.module_id == module_id && entry.profile_id == profile_id)?;

        Some(Arc::clone(&active.dir))
    })
}

/// One retained transient DIR for one in-flight build.
#[derive(Clone)]
pub(super) struct RetainedDirFrame {
    /// The build key that owns this transient DIR.
    pub build_key: BuildKey,
    /// The strongest read boundary this transient DIR currently satisfies.
    pub boundary: DirReadBoundary,
    /// The transient mutable DIR for the running build.
    pub dir: Arc<ModuleDir>,
}

/// One active transient DIR inside the current build frame.
#[derive(Clone)]
struct ActiveDirFrame {
    /// The module this transient DIR belongs to.
    module_id: ModuleId,
    /// The profile this transient DIR belongs to.
    profile_id: ProfileId,
    /// The strongest read boundary this transient DIR currently satisfies.
    boundary: DirReadBoundary,
    /// The transient mutable DIR for the running build.
    dir: Arc<ModuleDir>,
}

struct ActiveBaseDirFrame {
    /// The module this base DIR belongs to.
    module_id: ModuleId,
    /// The transient mutable base DIR for the running build.
    dir: ModuleDir,
}

/// The task-local mutable state for the current in-flight build.
#[derive(Default)]
struct BuildFrame {
    /// The currently active transient base DIR stack.
    active_base_dirs: Vec<ActiveBaseDirFrame>,
    /// The currently active transient DIR stack.
    active_dirs: Vec<ActiveDirFrame>,
}

impl Compiler {
    /// Return the best available frame that satisfies one read boundary.
    fn best_available_dir_frame<'a, T: 'a>(
        mut frames: impl DoubleEndedIterator<Item = &'a T>,
        boundary: DirReadBoundary,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary_of: impl Fn(&T) -> DirReadBoundary,
        module_of: impl Fn(&T) -> ModuleId,
        profile_of: impl Fn(&T) -> ProfileId,
        dir_of: impl Fn(&T) -> &Arc<ModuleDir>,
    ) -> Option<Arc<ModuleDir>> {
        let frame = frames.find(|frame| {
            module_of(frame) == module_id
                && profile_of(frame) == profile_id
                && boundary_of(frame) >= boundary
        })?;

        Some(Arc::clone(dir_of(frame)))
    }

    /// Panic when one active transient base DIR frame is unexpectedly missing.
    fn panic_missing_active_base_dir_frame(&self, module_id: Option<ModuleId>) -> ! {
        if let Some(module_id) = module_id {
            panic!("missing active base dir frame for {module_id:?}");
        }

        panic!("missing active base dir frame");
    }

    /// Return one current active transient base DIR frame when present.
    fn current_active_base_dir_entry<'a>(
        &self,
        frame: &'a BuildFrame,
        module_id: ModuleId,
    ) -> Option<&'a ActiveBaseDirFrame> {
        frame
            .active_base_dirs
            .iter()
            .rev()
            .find(|entry| entry.module_id == module_id)
    }

    /// Return the current active transient base DIR frame mutably when present.
    fn current_active_base_dir_entry_mut<'a>(
        &self,
        frame: &'a mut BuildFrame,
        module_id: ModuleId,
    ) -> Option<&'a mut ActiveBaseDirFrame> {
        frame
            .active_base_dirs
            .iter_mut()
            .rev()
            .find(|entry| entry.module_id == module_id)
    }

    /// Pop the current active transient base DIR from the build frame.
    fn pop_active_base_dir_frame(&self) -> ModuleDir {
        CURRENT_BUILD_FRAME.with(|frame| {
            let popped = frame.borrow_mut().active_base_dirs.pop();
            let Some(frame) = popped else {
                self.panic_missing_active_base_dir_frame(None);
            };

            frame.dir
        })
    }

    /// Pop the current active transient DIR from the build frame.
    fn pop_active_dir_frame(&self) {
        CURRENT_BUILD_FRAME.with(|frame| {
            let popped = frame.borrow_mut().active_dirs.pop();
            let Some(_) = popped else {
                panic!("missing active dir frame");
            };
        });
    }

    /// Return the current active transient DIR from one task-local build frame.
    fn current_thread_dir_frame(
        &self,
        frame: &BuildFrame,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
    ) -> Option<Arc<ModuleDir>> {
        Self::best_available_dir_frame(
            frame.active_dirs.iter().rev(),
            boundary,
            module_id,
            profile_id,
            |entry| entry.boundary,
            |entry| entry.module_id,
            |entry| entry.profile_id,
            |entry| &entry.dir,
        )
    }

    /// Return the current retained transient DIR for the running build.
    fn current_retained_dir_frame(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
    ) -> Option<Arc<ModuleDir>> {
        let current_build_key = self.current_build_key()?;
        let frames = self.retained_dir_frames.get(&(module_id, profile_id))?;
        Self::best_available_dir_frame(
            frames
                .iter()
                .rev()
                .filter(|frame| frame.build_key == current_build_key),
            boundary,
            module_id,
            profile_id,
            |frame| frame.boundary,
            |_| module_id,
            |_| profile_id,
            |frame| &frame.dir,
        )
    }

    /// Rebuild one transient mutable DIR builder from one committed artifact snapshot.
    pub(crate) fn transient_dir_builder_from_artifact(
        &self,
        snapshot: Arc<ModuleDirData>,
    ) -> ModuleDir {
        let snapshot = Arc::unwrap_or_clone(snapshot);

        ModuleDir::from_data(snapshot)
    }

    /// Rebuild one transient mutable DIR from one committed artifact snapshot.
    pub(crate) fn transient_dir_from_artifact(
        &self,
        snapshot: Arc<ModuleDirData>,
    ) -> Arc<ModuleDir> {
        Arc::new(self.transient_dir_builder_from_artifact(snapshot))
    }

    /// Return one transient DIR for one committed artifact snapshot.
    pub(crate) fn transient_dir_for_artifact(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
        snapshot: Arc<ModuleDirData>,
    ) -> Arc<ModuleDir> {
        if let Some(dir) = self.current_active_dir_frame(module_id, profile_id, boundary) {
            return dir;
        }

        self.transient_dir_from_artifact(snapshot)
    }

    /// Run one transient DIR build from one committed artifact snapshot.
    fn with_transient_artifact_dir_inner<T, E>(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
        snapshot: Arc<ModuleDirData>,
        is_shared: bool,
        handle: impl FnOnce(&Arc<ModuleDir>) -> Result<T, E>,
    ) -> Result<(T, ModuleDir), E> {
        // FUGU #Architecture: slice 3 still reconstructs transient mutable DIR builders
        // from committed snapshots before producers become directly artifact driven
        let dir = self.transient_dir_for_artifact(module_id, profile_id, boundary, snapshot);
        let result = if is_shared {
            self.with_active_dir_frame(module_id, profile_id, boundary, &dir, || handle(&dir))?
        } else {
            handle(&dir)?
        };

        let dir = if is_shared {
            self.finish_current_build_transient_dir(module_id, profile_id, dir)
        } else {
            self.finish_transient_dir(dir)
        };

        Ok((result, dir))
    }

    /// Run one transient DIR build that same-build reads may reuse.
    pub(crate) fn with_shared_transient_artifact_dir<T, E>(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
        snapshot: Arc<ModuleDirData>,
        handle: impl FnOnce(&Arc<ModuleDir>) -> Result<T, E>,
    ) -> Result<(T, ModuleDir), E> {
        self.with_transient_artifact_dir_inner(
            module_id, profile_id, boundary, snapshot, true, handle,
        )
    }

    /// Run one transient DIR build that stays private to the producer.
    pub(crate) fn with_private_transient_artifact_dir<T, E>(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
        snapshot: Arc<ModuleDirData>,
        handle: impl FnOnce(&Arc<ModuleDir>) -> Result<T, E>,
    ) -> Result<(T, ModuleDir), E> {
        self.with_transient_artifact_dir_inner(
            module_id, profile_id, boundary, snapshot, false, handle,
        )
    }

    /// Finish one transient DIR builder into an owned value.
    pub(crate) fn finish_transient_dir(&self, dir: Arc<ModuleDir>) -> ModuleDir {
        match Arc::try_unwrap(dir) {
            Ok(dir) => dir,
            Err(dir) => ModuleDir::from_data(dir.to_data()),
        }
    }

    /// Finish one current-build transient DIR, consuming the persisted frame first when present.
    pub(crate) fn finish_current_build_transient_dir(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: Arc<ModuleDir>,
    ) -> ModuleDir {
        let dir = self
            .take_retained_dir_frame_for_current_build(module_id, profile_id)
            .unwrap_or(dir);

        self.finish_transient_dir(dir)
    }

    /// Return the current build key for transient DIR ownership.
    fn current_frame_build_key(&self) -> BuildKey {
        self.current_build_key()
            .unwrap_or_else(|| panic!("missing current build key for transient DIR frame"))
    }

    /// Insert or replace one retained transient DIR for the current build.
    fn upsert_retained_dir_frame(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        new_boundary: DirReadBoundary,
        new_dir: &Arc<ModuleDir>,
    ) {
        let build_key = self.current_frame_build_key();
        let key = (module_id, profile_id);
        if let Some(mut entry) = self.retained_dir_frames.get_mut(&key) {
            let frames: &mut Vec<RetainedDirFrame> = entry.value_mut();
            if let Some(frame) = frames
                .iter_mut()
                .rev()
                .find(|frame| frame.build_key == build_key)
            {
                frame.boundary = new_boundary;
                frame.dir = Arc::clone(new_dir);
                return;
            }
        }

        self.retained_dir_frames
            .entry((module_id, profile_id))
            .or_default()
            .push(RetainedDirFrame {
                build_key,
                boundary: new_boundary,
                dir: Arc::clone(new_dir),
            });
    }

    /// Push one active transient DIR into the current task-local build frame.
    fn push_active_dir_frame(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
        dir: &Arc<ModuleDir>,
    ) {
        CURRENT_BUILD_FRAME.with(|frame| {
            frame.borrow_mut().active_dirs.push(ActiveDirFrame {
                module_id,
                profile_id,
                boundary,
                dir: Arc::clone(dir),
            });
        });
    }

    /// Clear all transient thread-local state from the current build frame.
    pub(crate) fn clear_current_build_frame(&self) {
        CURRENT_BUILD_FRAME.with(|frame| {
            let frame = &mut *frame.borrow_mut();
            frame.active_base_dirs.clear();
            frame.active_dirs.clear();
        });
    }

    /// Run one closure with one active transient base DIR in the current build frame.
    pub(crate) fn with_active_base_dir_frame<T>(
        &self,
        module_id: ModuleId,
        dir: ModuleDir,
        handle: impl FnOnce() -> T,
    ) -> (T, ModuleDir) {
        CURRENT_BUILD_FRAME.with(|frame| {
            frame
                .borrow_mut()
                .active_base_dirs
                .push(ActiveBaseDirFrame { module_id, dir });
        });

        let result = handle();
        let dir = self.pop_active_base_dir_frame();

        (result, dir)
    }

    /// Read the current active transient base DIR for one module.
    pub(crate) fn with_active_base_dir<T>(
        &self,
        module_id: ModuleId,
        handle: impl FnOnce(&ModuleDir) -> T,
    ) -> T {
        CURRENT_BUILD_FRAME.with(|frame| {
            let frame = frame.borrow();
            let active = self
                .current_active_base_dir_entry(&frame, module_id)
                .unwrap_or_else(|| self.panic_missing_active_base_dir_frame(Some(module_id)));

            handle(&active.dir)
        })
    }

    /// Read the current active transient base DIR for one module when present.
    pub(crate) fn with_current_active_base_dir<T>(
        &self,
        module_id: ModuleId,
        handle: impl FnOnce(&ModuleDir) -> T,
    ) -> Option<T> {
        CURRENT_BUILD_FRAME.with(|frame| {
            let frame = frame.borrow();
            let active = self.current_active_base_dir_entry(&frame, module_id)?;

            Some(handle(&active.dir))
        })
    }

    /// Mutate the current active transient base DIR for one module.
    pub(crate) fn with_active_base_dir_mut<T>(
        &self,
        module_id: ModuleId,
        handle: impl FnOnce(&mut ModuleDir) -> T,
    ) -> T {
        CURRENT_BUILD_FRAME.with(|frame| {
            let frame = &mut *frame.borrow_mut();
            let active = self
                .current_active_base_dir_entry_mut(frame, module_id)
                .unwrap_or_else(|| self.panic_missing_active_base_dir_frame(Some(module_id)));

            handle(&mut active.dir)
        })
    }

    /// Clear all retained transient DIR state for one completed build.
    pub(crate) fn clear_retained_build_frame(&self, build_key: &BuildKey) {
        let keys: Vec<_> = self
            .retained_dir_frames
            .iter()
            .map(|entry: dashmap::mapref::multiple::RefMulti<'_, _, _>| *entry.key())
            .collect();

        for key in keys {
            if let Some(mut entry) = self.retained_dir_frames.get_mut(&key) {
                let frames: &mut Vec<RetainedDirFrame> = entry.value_mut();
                frames.retain(|frame| &frame.build_key != build_key);

                if frames.is_empty() {
                    drop(entry);
                    self.retained_dir_frames.remove(&key);
                }
            }
        }
    }

    /// Take one retained transient DIR owned by the current build.
    pub(crate) fn take_retained_dir_frame_for_current_build(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<ModuleDir>> {
        let build_key = self.current_build_key()?;
        let key = (module_id, profile_id);
        let mut entry = self.retained_dir_frames.get_mut(&key)?;
        let frames: &mut Vec<RetainedDirFrame> = entry.value_mut();
        let mut dir = None;
        frames.retain(|frame| {
            if frame.build_key == build_key {
                dir = Some(Arc::clone(&frame.dir));
                return false;
            }

            true
        });

        if frames.is_empty() {
            drop(entry);
            self.retained_dir_frames.remove(&key);
        }

        dir
    }

    /// Run one closure with one active transient DIR in the current build frame.
    pub(crate) fn with_active_dir_frame<T>(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
        dir: &Arc<ModuleDir>,
        handle: impl FnOnce() -> T,
    ) -> T {
        self.push_active_dir_frame(module_id, profile_id, boundary, dir);
        self.upsert_retained_dir_frame(module_id, profile_id, boundary, dir);

        let result = handle();
        self.pop_active_dir_frame();

        result
    }

    /// Register or replace one active transient DIR in the current build frame.
    pub(crate) fn set_active_dir_frame(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
        dir: &Arc<ModuleDir>,
    ) {
        CURRENT_BUILD_FRAME.with(|frame| {
            let frame = &mut *frame.borrow_mut();
            if let Some(active) = frame
                .active_dirs
                .iter_mut()
                .rev()
                .find(|entry| entry.module_id == module_id && entry.profile_id == profile_id)
            {
                active.boundary = boundary;
                active.dir = Arc::clone(&dir);
            } else {
                frame.active_dirs.push(ActiveDirFrame {
                    module_id,
                    profile_id,
                    boundary,
                    dir: Arc::clone(&dir),
                });
            }
        });
        self.upsert_retained_dir_frame(module_id, profile_id, boundary, dir);
    }

    /// Return one snapshot for the strongest active transient DIR that satisfies this read.
    pub(crate) fn current_active_dir_frame(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        boundary: DirReadBoundary,
    ) -> Option<Arc<ModuleDir>> {
        let current = CURRENT_BUILD_FRAME.with(|frame| {
            let frame = frame.borrow();
            self.current_thread_dir_frame(&frame, module_id, profile_id, boundary)
        });
        if current.is_some() {
            return current;
        }

        self.current_retained_dir_frame(module_id, profile_id, boundary)
    }
}
