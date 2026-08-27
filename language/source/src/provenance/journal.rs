use destack_core::StringId;

use super::{ProvenanceBuilder, ProvenanceId};

/// A provenance journal for transforms bearing one stable name.
#[derive(Debug)]
pub struct ProvenanceJournal<'a> {
    /// The provenance table receiving transforms.
    builder: &'a mut ProvenanceBuilder,
    /// The interned transform name.
    name: StringId,
}

impl ProvenanceJournal<'_> {
    /// Bind one table builder to an interned transform name.
    pub(super) fn new(builder: &mut ProvenanceBuilder, name: StringId) -> ProvenanceJournal<'_> {
        ProvenanceJournal { builder, name }
    }

    /// Reborrow this journal for a nested transform walk.
    pub fn reborrow(&mut self) -> ProvenanceJournal<'_> {
        ProvenanceJournal {
            builder: self.builder,
            name: self.name,
        }
    }

    /// Produce provenance for one source expansion at another source site.
    pub fn expand(&mut self, source: ProvenanceId, site: ProvenanceId) -> ProvenanceId {
        let location = self.builder.insert_expanded(source, site);
        let [output] = self
            .builder
            .transform(self.name, &[source, site], [location]);

        output
    }

    /// Derive one provenance id at its source location.
    pub fn derive(&mut self, source: ProvenanceId) -> ProvenanceId {
        let location = self.builder.location_of(source);
        let [output] = self.builder.transform(self.name, &[source], [location]);

        output
    }

    /// Split one provenance id into several ids at its source location.
    pub fn split<const OUTPUTS: usize>(&mut self, source: ProvenanceId) -> [ProvenanceId; OUTPUTS] {
        let location = self.builder.location_of(source);

        self.builder
            .transform(self.name, &[source], [location; OUTPUTS])
    }

    /// Split one provenance id into a dynamic number of ids at its source location.
    pub fn split_many(&mut self, source: ProvenanceId, outputs: usize) -> Vec<ProvenanceId> {
        let location = self.builder.location_of(source);
        let locations = std::iter::repeat_n(location, outputs);

        self.builder.transform_many(self.name, &[source], locations)
    }

    /// Fuse all input locations into one provenance id.
    pub fn fuse(&mut self, inputs: &[ProvenanceId]) -> ProvenanceId {
        let location = self.builder.insert_fused(inputs);
        let [output] = self.builder.transform(self.name, inputs, [location]);

        output
    }

    /// Fuse all input locations into several provenance ids.
    pub fn fuse_many<const OUTPUTS: usize>(
        &mut self,
        inputs: &[ProvenanceId],
    ) -> [ProvenanceId; OUTPUTS] {
        let location = self.builder.insert_fused(inputs);

        self.builder
            .transform(self.name, inputs, [location; OUTPUTS])
    }

    /// Produce one compiler generated provenance id.
    pub fn generate(&mut self, inputs: &[ProvenanceId]) -> ProvenanceId {
        let location = self.builder.insert_generated();
        let [output] = self.builder.transform(self.name, inputs, [location]);

        output
    }

    /// Produce several compiler generated provenance ids.
    pub fn generate_many<const OUTPUTS: usize>(
        &mut self,
        inputs: &[ProvenanceId],
    ) -> [ProvenanceId; OUTPUTS] {
        let location = self.builder.insert_generated();

        self.builder
            .transform(self.name, inputs, [location; OUTPUTS])
    }

    /// Consume provenance without producing replacements.
    pub fn remove(&mut self, inputs: &[ProvenanceId]) {
        let _: [ProvenanceId; 0] = self.builder.transform(self.name, inputs, []);
    }
}
