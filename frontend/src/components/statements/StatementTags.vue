<script lang="ts" setup>
import { useAppearance } from "@/state/appearance";
import { useCurrentModule } from "@/state/module";
import { useStatementContext } from "@/state/statement";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { TagIcon as TagIconOutline } from "@heroicons/vue/24/outline";
import { TagIcon as TagIconSolid } from "@heroicons/vue/24/solid";
import { computed, nextTick, ref } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useOperations } from "@/state/operations";
import type { Tagging } from "@/gql/graphql";
import type { Statement } from "@/gql/graphql";
import { newTaggingId } from "@/state/operations/statement";
import FadeTransition from "@/components/basic/FadeTransition.vue";

const context = useStatementContext();
const module = useCurrentModule();
const appearance = useAppearance();
const ops = useOperations();

const addingTag = ref(false);
const popoverRef = ref<HTMLDivElement | null>(null);
const inputRef = ref<InstanceType<typeof ComboboxInput> | null>(null);
const query = ref("");
const popoverPin = pinAbsoluteElement(
  computed(() => popoverRef.value),
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
  nextTick(() => inputRef.value?.$el.focus());
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
  ops.symbol.softDeleteTagging(null, context.statement.value.id, tagging);
}

defineExpose({
  open,
  close,
});
</script>
<template>
  <div class="group relative flex flex-row gap-1.5">
    <!-- Existing tags -->
    <!-- obviously deleting on click is bad UX and will be fixed when we have proper tag value menus -->
    <button
      v-for="tagging in context.tags.value"
      :key="tagging.id"
      class="flex flex-row rounded-xl bg-yellow-100 px-1.5 text-orange-900 ring-1 ring-inset ring-yellow-600/20 hover:bg-yellow-200"
      @click="deleteTagging(tagging)"
    >
      <TagIconSolid class="mt-0.5 h-4 w-4" />
      <span class="text-orange-00 ml-0.5">{{ module.tagsByKey.value[tagging.key]?.name }}</span>
    </button>
    <!-- Add tag button -->
    <!-- Hidden for now because it interferes with triggers (makes spacing weird) -->
    <!-- <button
      v-if="!context.readonly.value"
      class="group/add flex flex-row rounded-xl border-gray-600 border-opacity-25 px-1 py-0 text-gray-400 hover:bg-yellow-100 hover:text-gray-700 group-hover/add:ring-1"
      :class="
        context.focused.value
          ? ''
          : 'opacity-0 transition-opacity duration-150 group-hover/statement:opacity-100 group-hover:opacity-100'
      "
      @click="open"
    >
      <TagIconOutline class="mt-0.5 h-4 w-4" />
      <PlusIcon class="ml-1 mt-0.5 h-4 w-4 opacity-0 transition-opacity duration-150 group-hover/add:opacity-100" />
    </button> -->
    <!-- Prevent scroll and capture click outside -->
    <div v-if="addingTag" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close()" />
    <!-- Add tag popover -->
    <FadeTransition>
      <div
        v-if="addingTag"
        ref="popoverRef"
        class="z-50 flex w-72 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="[popoverPin.pinned.value ? '' : 'absolute -top-9']"
      >
        <Combobox as="div" @update:model-value="(t) => (createTagging(t), close())">
          <!-- Title -->
          <h5 class="text-left text-sm font-semibold text-gray-900">
            Add tag to {{ context.statement.value.name ?? "statement" }}
          </h5>
          <!-- Input -->
          <ComboboxInput
            as="input"
            ref="inputRef"
            class="mt-1 w-full rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:ring-0"
            @change="query = $event.target.value"
            @keydown.enter.prevent.stop="close"
            @keydown.escape.prevent.stop="close"
            :class="{
              'font-mono': appearance.fontMono,
              'text-sm': appearance.textSmall,
              'text-md': !appearance.textSmall,
            }"
          >
          </ComboboxInput>
          <!-- Tag options -->
          <ComboboxOptions
            class="mt-1 max-h-48 overflow-auto"
            static
            :class="{
              'font-mono': appearance.fontMono,
              'text-sm': appearance.textSmall,
              'text-md': !appearance.textSmall,
            }"
          >
            <ComboboxOption v-for="tag in filteredTags" :key="tag.id" :value="tag" v-slot="{ active, selected }">
              <li
                :class="[
                  'relative flex cursor-default select-none flex-col px-1 py-[3px] text-gray-900',
                  active ? 'bg-orange-100' : '',
                  selected ? 'text-orange-900' : '',
                ]"
              >
                <!-- Tag path -->
                <div class="flex items-center justify-between">
                  <span class="flex flex-row items-center">
                    <TagIconOutline class="h-4 w-4 text-gray-400" />
                    <span class="ml-1 text-gray-900">{{ tag.name }}</span>
                  </span>
                  <!-- Source -->
                  <span class="text-xs" :class="['truncate', active ? 'text-gray-700' : 'text-gray-500']">
                    {{ module.pathOf(tag.file) ?? "(builtin)" }}
                  </span>
                </div>
                <!-- Tag description -->
                <span
                  class="ml-5 max-w-full truncate text-xs"
                  :class="['', active ? 'text-gray-700' : 'text-gray-500']"
                >
                  {{ tag.description }}
                </span>
              </li>
            </ComboboxOption>
          </ComboboxOptions>
        </Combobox>
      </div>
    </FadeTransition>
  </div>
</template>
