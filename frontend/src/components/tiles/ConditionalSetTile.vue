<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import SelectFieldInterface from "@/components/interfaces/SelectFieldInterface.vue";
import ConditionalTile from "@/components/tiles/ConditionalTile.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import type { Conditional } from "@/gql/graphql";
import { getDefaultConditional } from "@/state/database";
import type { Field, Statement } from "@/state/module";
import { PlusIcon } from "@heroicons/vue/24/solid";
import { computed, ref } from "vue";

const props = defineProps<{ fields: Field[]; modelValue?: Conditional[] }>();
const emit = defineEmits<{ (e: "update:modelValue", value?: Conditional[]): void }>();

const fieldsByCk = computed(() => {
  const map: Record<string, Field> = {};
  for (const field of props.fields) {
    map[field.ck] = field;
  }
  return map;
});

function updateConditional(index: number, conditional?: Conditional) {
  const value = [...(props.modelValue ?? [])];
  if (conditional == null) {
    value.splice(index, 1);
  } else {
    value[index] = conditional;
  }
  emit("update:modelValue", value);
}

function addDefaultConditional(field: Field) {
  const value = [...(props.modelValue ?? [])];
  value.push(getDefaultConditional(field));
  emit("update:modelValue", value);
}

const adding = ref(false);
const popoverRef = ref<HTMLDivElement | null>(null);
const popoverPin = pinAbsoluteElement(popoverRef, { pos: true, keepInView: true });

function close() {
  adding.value = false;
}

function open() {
  adding.value = true;
}

defineExpose({
  open,
  close,
  addDefaultConditional,
});
</script>
<template>
  <div class="relative flex flex-row gap-1">
    <!-- Filters -->
    <!-- TODO @UX: show Notion-like potential filters for selected fields -->
    <ConditionalTile
      v-for="(conditional, i) in modelValue?.filter((c) => fieldsByCk[c.field as string] != null) ?? []"
      :key="i"
      :field="fieldsByCk[conditional.field as string]"
      :modelValue="conditional"
      @update:modelValue="($event) => updateConditional(i, $event)"
    />
    <!-- Arbitary filters -->
    <button class="flex flex-row items-center rounded-sm px-0.5 py-0.5 text-gray-400 hover:bg-amber-100" @click="open">
      <PlusIcon class="h-4 w-4" />
      Filter
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
        <div class="px-1 text-sm text-gray-700">
          <span>Filter by</span>
        </div>
        <SelectFieldInterface
          class="mt-1"
          :options="fields"
          @update:modelValue="($event) => (addDefaultConditional($event), close())"
        />
      </div>
    </FadeTransition>
  </div>
</template>
