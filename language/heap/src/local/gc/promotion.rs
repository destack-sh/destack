use destack_mir::ReferenceMap;

use crate::allocator::SpanSlot;
use crate::local::space::{HeapLocation, HeapPlace, HeapSpace, YoungPlace};
use crate::{HeapError, HeapReference, HeapResult, RootSet, RootSlot, heap_reference_offsets};

/// One planned relocation for a heap reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Promotion {
    /// The reference whose physical place moves.
    pub(crate) reference: HeapReference,
    /// The source physical location.
    pub(crate) source: HeapPlace,
    /// The target physical location.
    pub(crate) target: HeapPlace,
}

impl HeapSpace {
    /// Promote one young reference into mature space.
    pub(crate) fn promote_reference(
        &mut self,
        reference: HeapReference,
    ) -> HeapResult<HeapReference> {
        // resolve the current live location
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        // stage the young to mature relocation
        let mut promotions = Vec::with_capacity(1);
        match location.place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                self.stage_young_promotion(reference, first_offset, &mut promotions)?;
            }
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                self.stage_young_run_promotion(reference, slot, &mut promotions)?;
            }
            _ => return Ok(reference),
        }

        // publish the mature location only after staging succeeds
        if let Err(error) = self.commit_young_promotions(&promotions) {
            self.discard_young_promotions(&promotions)?;

            return Err(error);
        }

        // retire the old nursery source after the promoted reference points at mature place
        match location.place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((allocation_index, _allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };
                self.young.live.clear(allocation_index);
            }
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                let Some(bits) = self.young.run_bits_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                bits.freed.set(slot.slot_index());
            }
            _ => {}
        }

        // remember the new mature location conservatively
        let Some(promotion) = promotions.first().copied() else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        // record mature write metadata for the promoted payload
        let promoted_reference = self.base_reference(promotion.target)?;
        let result = self.record_write_barrier(
            promoted_reference,
            HeapLocation {
                place: promotion.target,
                base: promoted_reference,
                byte_offset: 0,
                byte_len: location.byte_len,
            },
            0,
            location.byte_len,
        );
        self.young.clear_forwarding();
        result?;

        Ok(promoted_reference)
    }

    /// Rewrite roots and traced mature payloads through one completed promotion set.
    pub(super) fn rewrite_promoted_references<R>(
        &mut self,
        roots: &mut R,
        promotions: &[Promotion],
    ) -> Result<(), R::Error>
    where
        R: RootSet,
    {
        if promotions.is_empty() {
            return Ok(());
        }

        // rewrite root slots first
        roots.visit_root_slots(&mut |mut slot: RootSlot<'_>| {
            let reference = slot.load()?;

            if let Some(next_reference) = self.forwarded_reference(reference)? {
                slot.store(next_reference)?;
            }

            Ok(())
        })?;

        // then rewrite every pinned reference
        let mut pins = std::mem::take(&mut self.pins);
        let rewrite_result = pins.rewrite_with(|reference| self.forwarded_reference(reference));
        self.pins = pins;
        rewrite_result?;

        // then rewrite every mature payload that may still contain young references
        self.rewrite_live_heap_references()?;

        // finally rebuild the auxiliary shared-edge tracking over the new stable refs
        self.rebuild_shared_edge_roots()?;
        self.shared_edge_cursor = 0;
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();

        Ok(())
    }

    /// Stage one fixed-size young slot relocation into mature space.
    pub(super) fn stage_young_run_promotion(
        &mut self,
        reference: HeapReference,
        slot: SpanSlot,
        promotions: &mut Vec<Promotion>,
    ) -> HeapResult<()> {
        // resolve the young run and source bytes
        let Some(run) = self.young.run(slot.span_index()).cloned() else {
            return Err(HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(HeapError::MissingSpan {
                    span_index: slot.span_index(),
                }),
            });
        };
        let byte_len = run.size_class;
        let source_offset = run.slot_offset(slot.slot_index());
        let bytes = self
            .mapping
            .bytes(source_offset, byte_len)
            .map_err(|error| HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(error.into()),
            })?;
        let reference_map = ReferenceMap::None;

        // keep fixed-size no-scan slots in small space when possible
        let location = if self.small.size_classes.class_index_for(byte_len).is_some() {
            let slot = self
                .allocate_small_payload_from_bytes(&bytes, &reference_map, false)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?
                .ok_or(HeapError::HeapPromotionUnavailableSmallSlot {
                    reference,
                    byte_len,
                })?;

            HeapPlace::Small(slot)
        } else {
            let pages = self.allocate_page_run(byte_len).map_err(|error| {
                HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                }
            })?;
            let allocation_id = self
                .insert_large_allocation(
                    byte_len,
                    self.allocator().page_bytes(),
                    pages,
                    reference_map,
                    false,
                )
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;
            let Some(allocation) = self.large_allocation(allocation_id) else {
                return Err(HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    }),
                });
            };
            self.mapping
                .copy_bytes(allocation.first_offset, &bytes)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error.into()),
                })?;

            HeapPlace::Large(allocation_id)
        };

        // publish forwarding metadata for later root rewriting
        let promoted_reference = self.base_reference(location)?;
        self.young.forward_run_slot(slot, promoted_reference);

        promotions.push(Promotion {
            reference,
            source: HeapPlace::Young(YoungPlace::Slot(slot)),
            target: location,
        });

        Ok(())
    }

    /// Stage one live young allocation relocation into mature space.
    pub(super) fn stage_young_promotion(
        &mut self,
        reference: HeapReference,
        first_offset: usize,
        promotions: &mut Vec<Promotion>,
    ) -> HeapResult<()> {
        // resolve the young allocation and source bytes
        let Some((allocation_index, allocation)) = self.young_range_by_offset(first_offset) else {
            return Err(HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(HeapError::MissingYoungRange { first_offset }),
            });
        };

        // copy the young bytes before relocating the allocation
        let young_offset = allocation.first_offset;
        let bytes = self
            .mapping
            .bytes(young_offset, allocation.byte_len)
            .map_err(|error| HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(error.into()),
            })?;
        let reference_map = self
            .young_range_reference_map(first_offset)
            .map_err(|error| HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(error),
            })?;

        // keep small mature payloads in size-class spans
        let location = if self
            .small
            .size_classes
            .class_index_for(allocation.byte_len)
            .is_some()
        {
            let slot = self
                .allocate_small_payload_from_bytes(&bytes, &reference_map, false)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;
            let Some(slot) = slot else {
                return Err(HeapError::HeapPromotionUnavailableSmallSlot {
                    reference,
                    byte_len: allocation.byte_len,
                });
            };

            HeapPlace::Small(slot)
        } else {
            let pages = self
                .allocate_page_run(allocation.byte_len)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;

            let allocation_id = self
                .insert_large_allocation(
                    allocation.byte_len,
                    self.allocator().page_bytes(),
                    pages,
                    reference_map,
                    false,
                )
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;
            let Some(allocation) = self.large_allocation(allocation_id) else {
                return Err(HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    }),
                });
            };
            self.mapping
                .copy_bytes(allocation.first_offset, &bytes)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error.into()),
                })?;

            HeapPlace::Large(allocation_id)
        };

        // publish forwarding metadata for later root rewriting
        let promoted_reference = self.base_reference(location)?;
        self.young
            .forward_range(allocation_index, promoted_reference);

        promotions.push(Promotion {
            reference,
            source: HeapPlace::Young(YoungPlace::Range { first_offset }),
            target: location,
        });

        Ok(())
    }

    /// Verify every staged young relocation still refers to the staged source.
    pub(super) fn commit_young_promotions(&mut self, promotions: &[Promotion]) -> HeapResult<()> {
        // verify every source before committing root rewrites
        for promotion in promotions {
            let reference = promotion.reference;
            let Some(location) = self.resolve_location(reference) else {
                return Err(HeapError::InvalidHeapReference { reference });
            };

            if location.place != promotion.source {
                return Err(HeapError::InvalidHeapReference { reference });
            }
        }

        Ok(())
    }

    /// Discard every staged mature relocation target.
    pub(super) fn discard_young_promotions(&mut self, promotions: &[Promotion]) -> HeapResult<()> {
        // release targets in reverse staging order
        for promotion in promotions.iter().rev() {
            let reference = promotion.reference;
            let result = match promotion.target {
                HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                    Err(HeapError::MissingYoungRange { first_offset })
                }
                HeapPlace::Young(YoungPlace::Slot(slot)) => Err(HeapError::MissingSpan {
                    span_index: slot.span_index(),
                }),
                HeapPlace::Small(slot) => self.release_small_slot(slot),
                HeapPlace::Large(allocation_id) => {
                    let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                        return Err(HeapError::MissingLargeAllocation {
                            allocation_id: allocation_id.id(),
                        });
                    };

                    if !allocation.is_live {
                        return Err(HeapError::MissingLargeAllocation {
                            allocation_id: allocation_id.id(),
                        });
                    }

                    let pages = allocation.pages;
                    allocation.retire();
                    self.large
                        .free_large_allocation_ids
                        .push(allocation_id.id());

                    // release the unpublished target pages after discarding the slot
                    self.release_page_run(pages)?;

                    Ok(())
                }
            };

            result.map_err(|error| HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(error),
            })?;
        }

        // clear forwarding metadata after every target is gone
        self.young.clear_forwarding();

        Ok(())
    }

    /// Return the promoted reference for one young reference.
    fn forwarded_reference(&self, reference: HeapReference) -> HeapResult<Option<HeapReference>> {
        // non-live references have no forwarding state
        let Some(location) = self.resolve_location(reference) else {
            return Ok(None);
        };

        // only young places can be forwarded
        let reference = match location.place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((start_index, _range)) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };

                self.young.forwarded_range(start_index)
            }
            HeapPlace::Young(YoungPlace::Slot(slot)) => self.young.forwarded_run_slot(slot),
            _ => None,
        };

        Ok(reference)
    }

    /// Rewrite every live mature payload through completed forwarding metadata.
    fn rewrite_live_heap_references(&mut self) -> HeapResult<()> {
        // snapshot live references before rewriting payloads
        let live_references = self.live_references()?;

        for reference in live_references {
            let Some(location) = self.resolve_location(reference) else {
                continue;
            };

            if matches!(
                location.place,
                HeapPlace::Young(YoungPlace::Range { .. }) | HeapPlace::Young(YoungPlace::Slot(_))
            ) {
                continue;
            }

            // noscan payloads cannot contain forwarding references
            let reference_map = self.reference_map_for_place(location.place)?;
            if !reference_map.has_local_reference() {
                continue;
            }

            self.rewrite_location_heap_references(location, &reference_map)?;
        }

        Ok(())
    }

    /// Rewrite one traced heap payload through completed forwarding metadata.
    fn rewrite_location_heap_references(
        &mut self,
        location: HeapLocation,
        reference_map: &ReferenceMap,
    ) -> HeapResult<()> {
        let base_address = self.mapping.base_address() + location.base.offset();
        let offsets = heap_reference_offsets(reference_map, base_address)?;

        for offset in offsets {
            self.rewrite_location_heap_reference_word(location, offset)?;
        }

        Ok(())
    }

    /// Rewrite one direct heap-reference word inside one traced payload.
    fn rewrite_location_heap_reference_word(
        &mut self,
        location: HeapLocation,
        start: usize,
    ) -> HeapResult<()> {
        let mut window = [0u8; HeapReference::BYTE_LEN];

        // read the current reference word
        self.fill_location_bytes(location, start, &mut window)?;

        let reference = HeapReference::from_bits(usize::from_le_bytes(window));
        let Some(next_reference) = self.forwarded_reference(reference)? else {
            return Ok(());
        };

        // overwrite only forwarded references
        let next_bytes = next_reference.bits().to_le_bytes();

        self.write_location_bytes(location, start, &next_bytes)
    }
}
