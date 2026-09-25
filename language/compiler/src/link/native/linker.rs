use tspp_native as native;
use tspp_program::Object;

use crate::LinkResult;

use super::super::program::{FrameLinker, ProgramLinker, ProgramStatics};
use super::Image;

/// Link relocatable native objects into one position-independent code image.
#[derive(Debug)]
pub(crate) struct NativeLinker<'a, 'b> {
    /// Program linker owning final runtime identities.
    pub(super) program: &'a ProgramLinker<'b>,
    /// Linked canonical frame-state identities.
    pub(super) frames: &'a FrameLinker<'b>,
    /// Linked static storage offsets.
    pub(super) statics: &'a ProgramStatics,
}

impl<'a, 'b> NativeLinker<'a, 'b> {
    /// Create one native linker.
    pub(crate) const fn new(
        program: &'a ProgramLinker<'b>,
        frames: &'a FrameLinker<'b>,
        statics: &'a ProgramStatics,
    ) -> Self {
        Self {
            program,
            frames,
            statics,
        }
    }

    /// Link native code when every object carries that code form.
    pub(crate) fn link(&self) -> LinkResult<Option<native::CodeBuilder>> {
        // inspect native-code coverage across the complete Program
        let object_count = self.program.objects().len();
        let native_count = self
            .program
            .objects()
            .iter()
            .filter(|(_, object)| object.native().is_some())
            .count();

        // omit native code only when no object carries it
        if native_count == 0 {
            return Ok(None);
        }

        // reject incomplete code forms
        if native_count != object_count {
            return Err(self
                .program
                .invalid_input("only some linked objects contain native code"));
        }

        // place and relocate every object into one executable image
        let (target, features) = self.target()?;
        let mut image = Image::default();

        // place independently aligned code blocks
        self.place_blocks(&mut image)?;

        // assign physical frame maps before resolving their index cells
        let traps = self.link_traps(&image)?;
        let (frames, constants) = self.link_maps(&mut image)?;
        self.place_indices(&mut image)?;
        self.place_imports(&mut image, target)?;

        // project callable entries and patch every code relocation
        let functions = self.link_functions(&image)?;
        let resumes = self.link_resumes(&image)?;
        self.patch_blocks(&mut image, &functions)?;
        let (unwind, personality_relocations) = self.link_unwind(&image, &functions)?;
        image.import_relocations.extend(personality_relocations);

        let target = self.program.intern_string(target);
        let features = features
            .into_iter()
            .map(|feature| self.program.intern_string(&feature));
        let map = native::CodeMapBuilder::new()
            .traps(traps)
            .frames(frames)
            .constants(constants);
        let imports = image.import_relocations;
        let alignment = image.alignment;
        let bytes = image.bytes;
        let mut code = native::CodeBuilder::new(target)
            .features(features)
            .bytes(bytes, alignment)
            .functions(functions)
            .resumes(resumes)
            .imports(imports)
            .map(map);
        if let Some(unwind) = unwind {
            code = code.unwind(unwind);
        }

        Ok(Some(code))
    }

    /// Return the common native target and feature set.
    pub(super) fn target(&self) -> LinkResult<(&str, Vec<String>)> {
        let Some((_, first)) = self.program.objects().first() else {
            return Err(self.program.invalid_input("Program has no native objects"));
        };
        let first = self.source(first)?;
        let target = first.target();
        let features = first.features().map(str::to_owned).collect::<Vec<_>>();

        // require one exact native ABI across every linked object
        for (module, object) in self.program.objects() {
            let source = self.source(object)?;
            let source_features = source.features().collect::<Vec<_>>();
            if source.target() != target
                || source.target_layout() != self.program.target_layout()
                || source_features != features.iter().map(String::as_str).collect::<Vec<_>>()
            {
                return Err(self
                    .program
                    .invalid_input(format!("module {module:?} uses a different native target")));
            }
        }

        Ok((target, features))
    }

    /// Return the native code emitted for one object.
    pub(super) fn source<'c>(&self, object: &'c Object) -> LinkResult<&'c native::Object> {
        object
            .native()
            .ok_or_else(|| self.program.invalid_input("object has no native code"))
    }
}
