<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import SelectFieldInterface from "@/components/interfaces/SelectFieldInterface.vue";
import SortTile from "@/components/tiles/SortTile.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { SortOp, type Sort } from "@/gql/graphql";
import type { Field } from "@/state/module";
import { ArrowUpIcon, PlusIcon } from "@heroicons/vue/24/solid";
import { computed, ref } from "vue";

const props = defineProps<{
  fields: Field[];
  modelValue?: Sort[];
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value?: Sort[]): void;
}>();

const fieldsByCk = computed(() => {
  const map: Record<string, Field> = {};
  for (const field of props.fields) {
    map[field.ck] = field;
  }
  return map;
});

function updateSort(index: number, sort?: Sort) {
  const value = [...(props.modelValue ?? [])];
  if (sort == null) {
    value.splice(index, 1);
  } else {
    value[index] = sort;
  }
  emit("update:modelValue", value);
}

function addSort(field: Field) {
  const value = [...(props.modelValue ?? [])];
  value.push({ field: field.ck, order: addingOrder.value });
  emit("update:modelValue", value);
}

const adding = ref(false);
const addingOrder = ref(SortOp.Ascending);
const popoverRef = ref<HTMLDivElement | null>(null);
const popoverPin = pinAbsoluteElement(popoverRef, { pos: true, keepInView: true });

function close() {
  adding.value = false;
}

function open() {
  adding.value = true;
}
</script>
<template>
  <div class="relative flex flex-row gap-1">
    <!-- Sorts -->
    <SortTile
      v-for="(sort, i) in modelValue?.filter((s) => fieldsByCk[s.field] != null) ?? []"
      :key="i"
      :field="fieldsByCk[sort.field]"
      :modelValue="sort"
      @update:modelValue="($event) => updateSort(i, $event)"
    />
    <!-- Add sort -->
    <button
      class="relative flex flex-row items-center rounded-sm px-0.5 py-0.5 text-gray-400 hover:bg-amber-100"
      @click="open"
    >
      <PlusIcon class="h-4 w-4" />
      Sort
    </button>
    <!-- Prevent scroll and capture click outside -->
    <div v-if="adding" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close()" />
    <!-- Add popover -->
    <FadeTransition>
      <div
        v-if="adding"
        ref="popoverRef"
        class="z-50 flex w-60 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="[popoverPin.pinned.value ? '' : 'absolute -right-48 top-7']"
        @keydown.escape.exact.prevent.stop="close()"
      >
        <div class="flex flex-row items-baseline px-1 text-sm text-gray-700">
          <span>Sort</span>
          <button
            class="mx-1 flex flex-row items-center rounded-sm border-orange-900/[12%] px-0.5 text-gray-700 hover:bg-amber-100"
            @click="addingOrder = addingOrder == SortOp.Ascending ? SortOp.Descending : SortOp.Ascending"
          >
            <span>{{ addingOrder == SortOp.Ascending ? "ascending" : "descending" }}</span>
            <ArrowUpIcon
              class="ml-0.5 h-4 w-4 transform transition-transform"
              :class="addingOrder == SortOp.Ascending ? 'rotate-180' : 'rotate-0'"
            />
          </button>
          <span>by</span>
        </div>
        <SelectFieldInterface
          class="mt-1"
          :options="fields"
          @update:modelValue="($event) => (addSort($event), close())"
        />
      </div>
    </FadeTransition>
  </div>
</template>
