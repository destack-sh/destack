<script lang="ts" setup>
import { useAppearance } from "@/state/appearance";
import { useCurrentModule } from "@/state/module";
import { useStatementContext } from "@/state/statement";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { TagIcon, XMarkIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useFocus } from "@vueuse/core";
import { useOperations } from "@/state/operations";
import type { Tagging } from "@/gql/graphql";
import type { Statement } from "@/gql/graphql";
import { newTaggingId } from "@/state/operations/statement";

const context = useStatementContext();
const module = useCurrentModule();
const appearance = useAppearance();
const ops = useOperations();

const addingTag = ref(false);
const popoverRef = ref<InstanceType<typeof Combobox> | null>(null);
const inputRef = ref<InstanceType<typeof ComboboxInput> | null>(null);
const inputRefFocused = useFocus(computed(() => inputRef.value?.$el));
const query = ref("");
const popoverPin = pinAbsoluteElement(
  computed(() => popoverRef.value?.$el),
  { pos: true, width: true, keepInView: true }
);

const uf = new uFuzzy({ intraMode: 0 });
const filteredTags = computed(() => {
  const unassigned = module.tags.value.filter((t) => !context.tags.value.some((t2) => t2.key == t.key));
  if (query.value.trim() == "") return unassigned;
  const [idxs] = uf.search(
    unassigned.map((t) => t.name ?? ""),
    query.value
  );
  return idxs?.map((idx) => unassigned[idx]) ?? [];
});

function open() {
  addingTag.value = true;
  nextTick(() => (inputRefFocused.focused.value = true));
}

function close() {
  addingTag.value = false;
}

function createTagging(tag: Pick<Statement, "id" | "key" | "name">) {
  ops.symbol.createTagging(null, context.statement.value.id, {
    id: newTaggingId(),
    key: tag.key as string,
    reference: { id: tag.id } as any,
    metadata: null,
  });
}

function deleteTagging(tagging: Pick<Tagging, "id">) {
  ops.symbol.softDeleteTagging(null, context.statement.value.id, tagging.id);
}

defineExpose({
  open,
  close,
});
</script>
<template>
  <div class="group relative flex flex-row gap-1.5">
    <!-- Existing tags -->
    <button
      v-for="tagging in context.tags.value"
      :key="tagging.id"
      class="flex flex-row items-center rounded-xl border-orange-600 px-0.5 hover:bg-orange-100"
    >
      <TagIcon class="h-4 w-4 text-orange-600" />
      <span class="ml-0.5 text-orange-600">{{ module.tagsByKey.value[tagging.key]?.name }}</span>
    </button>
    <!-- Add tag button -->
    <button
      v-if="!context.readonly.value"
      class="rounded-sm p-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700"
      :class="context.focused.value ? '' : 'opacity-0 transition-opacity group-hover:opacity-100'"
      @click="open"
    >
      <TagIcon class="h-4 w-4" />
    </button>
    <!-- Prevent scroll and capture click outside -->
    <div v-if="addingTag" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close()" />
    <!-- Add tag popover -->
    <Combobox
      v-if="addingTag"
      ref="popoverRef"
      as="div"
      class="z-50 flex w-72 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="[popoverPin.pinned.value ? '' : 'absolute -top-9']"
      @update:model-value="(t) => (createTagging(t), close())"
    >
      <!-- Title -->
      <h5 class="text-left text-sm font-semibold text-gray-900">
        Add tag to {{ context.statement.value.name ?? "statement" }}
      </h5>
      <!-- Input -->
      <ComboboxInput
        as="input"
        ref="inputRef"
        class="w-full rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:ring-0"
        @change="query = $event.target.value"
        @keydown.enter.prevent.stop="close"
        @keydown.escape.prevent.stop="close"
      >
      </ComboboxInput>
      <!-- Tag options -->
      <ComboboxOptions
        class="mt-1 max-h-48 overflow-auto"
        static
        :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
      >
        <ComboboxOption v-for="tag in filteredTags" :key="tag.id" :value="tag" v-slot="{ active, selected }">
          <li
            :class="[
              'relative flex cursor-default select-none flex-col px-1 py-[3px] text-gray-900',
              active ? 'bg-orange-100' : '',
              selected ? 'text-orange-600' : '',
            ]"
          >
            <!-- Tag path -->
            <div class="flex items-baseline justify-between">
              <span class="flex flex-row items-center">
                <TagIcon class="h-4 w-4 text-orange-600" />
                <span class="ml-1 font-semibold text-orange-600">{{ tag.name }}</span>
              </span>
              <!-- Source -->
              <span class="text-xs" :class="['truncate', active ? 'text-gray-700' : 'text-gray-500']">(builtin)</span>
            </div>
            <!-- Tag description -->
            <span class="text-xs" :class="['', active ? 'text-gray-700' : 'text-gray-500']">
              {{ tag.description }}
            </span>
          </li>
        </ComboboxOption>
      </ComboboxOptions>
    </Combobox>
  </div>
</template>
