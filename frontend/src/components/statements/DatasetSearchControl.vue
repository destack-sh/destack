<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useElementPanelSettings } from "@/state/bench";
import type { DatasetStatementProperties } from "@/state/statement";
import { MagnifyingGlassIcon, XCircleIcon } from "@heroicons/vue/24/outline";
import { nextTick, ref, toRef } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "focused" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const searchRef = ref<InstanceType<typeof EditableSpan> | null>(null);
const properties = useElementPanelSettings<DatasetStatementProperties>(toRef(props, "statement"), {
  inlineQuery: undefined,
  wrapColumns: false,
});

function toggleInlineSearch() {
  if (properties.inlineQuery == null) {
    properties.inlineQuery = "";
    nextTick(() => searchRef.value?.focus());
  } else {
    properties.inlineQuery = undefined;
    emit("navigateLeft");
  }
}

defineExpose({
  focus: (f: "first" | "last" = "first") => {
    if (properties.inlineQuery == null) {
      toggleInlineSearch();
    } else {
      searchRef.value?.focus();
    }
  },
  blur: () => {
    searchRef.value?.blur();
  },
  actions: [
    {
      label: "Search",
      groupId: "nav",
      icon: MagnifyingGlassIcon,
      action: () => {
        properties.inlineQuery = "";
        nextTick(() => searchRef.value?.focus());
      },
      hideInline: true,
    },
  ],
});
</script>
<template>
  <!-- Quick inline search -->
  <div class="inline-flex flex-row">
    <button
      tabindex="-1"
      class="mb-0.5 rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
      @click="() => toggleInlineSearch()"
    >
      <MagnifyingGlassIcon class="h-4 w-4" />
    </button>
    <div
      v-if="(focused && properties.inlineQuery != null) || (properties.inlineQuery ?? '').length > 0"
      class="relative h-full w-40 transition-transform duration-150"
      @click="searchRef?.focus()"
    >
      <EditableSpan
        ref="searchRef"
        :class="focused ? '' : 'h-0'"
        :model-value="properties.inlineQuery ?? ''"
        :readonly="false"
        @update:model-value="(v) => (properties.inlineQuery = v)"
        @keydown.escape.exact.prevent="toggleInlineSearch"
        class="overflow-hidden whitespace-nowrap"
        placeholder
      />
      <!-- Placeholder -->
      <span class="text-gray-400" v-if="(properties.inlineQuery ?? '').trim() == ''">Type to search...</span>
      <!-- Cancel button -->
      <button
        tabindex="-1"
        v-if="(properties.inlineQuery ?? '').trim() != ''"
        class="absolute right-0 top-0 h-full rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
        @click="() => (properties.inlineQuery = undefined)"
      >
        <XCircleIcon class="h-4 w-4" />
      </button>
    </div>
  </div>
</template>
