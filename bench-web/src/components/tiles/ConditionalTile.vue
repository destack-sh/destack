<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import TypePreview from "@/components/interfaces/TypePreview.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { ConditionalOp, type Conditional } from "@/gql/graphql";
import { CONDITIONAL_OP_NAME, EXPRESSION_OPS, getSupportedConditionalOps } from "@/state/database";
import type { Field } from "@/state/module";
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { ChevronUpDownIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { computed, ref } from "vue";

const props = defineProps<{ field: Field; modelValue: Conditional }>();
const emit = defineEmits<{ (e: "update:modelValue", value?: Conditional): void }>();

const editing = ref(false);
const popoverRef = ref<HTMLDivElement | null>(null);
const popoverPin = pinAbsoluteElement(popoverRef, { pos: true, keepInView: true });

const RENDERABLE_OPS: ConditionalOp[] = [
  ...EXPRESSION_OPS.COND_EXISTENCE,
  ...[ConditionalOp.Equals, ConditionalOp.NotEquals], // should be COND_EXACT but we don't support set ops (in/not in) yet
  ...EXPRESSION_OPS.COND_RANGE,
  ...EXPRESSION_OPS.COND_STRING,
];
const availableOps = computed(() =>
  getSupportedConditionalOps(props.field).filter((op) => RENDERABLE_OPS.includes(op))
);
const requiresScalar = computed(() =>
  [...EXPRESSION_OPS.COND_EXACT, ...EXPRESSION_OPS.COND_RANGE, ...EXPRESSION_OPS.COND_STRING].includes(
    props.modelValue.op
  )
);

function close() {
  editing.value = false;
}
function open() {
  editing.value = true;
}
</script>
<template>
  <button
    class="flex flex-row items-center rounded-xl border border-amber-900/[12%] px-1.5 py-0.5 hover:bg-amber-100"
    @click="open()"
  >
    <TypePreview :type="field" />
    <span class="ml-1 text-gray-900 underline decoration-gray-300 underline-offset-4">{{ field.name }}</span
    >:
    <span class="ml-1.5">{{ CONDITIONAL_OP_NAME[props.modelValue.op] }}</span>
    <ValueInterface
      v-if="requiresScalar"
      class="ml-0.5"
      :type="field"
      readonly
      :model-value="props.modelValue.value"
      @focus="open()"
    />
  </button>
  <!-- Prevent scroll and capture click outside -->
  <div v-if="editing" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close()" />
  <!-- Edit popover -->
  <FadeTransition>
    <div
      v-if="editing"
      ref="popoverRef"
      class="z-50 flex w-60 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="[popoverPin.pinned.value ? '' : 'absolute left-0 top-7']"
      @keydown.escape.exact.prevent.stop="close()"
    >
      <div class="flex flex-row items-center">
        <!-- Field (can't change here) -->
        <span
          class="border border-orange-900/[12%] px-1.5 py-0.5 font-bold underline decoration-gray-300 underline-offset-4"
          >{{ field.name }}</span
        >
        <!-- Operator select -->
        <Listbox
          as="div"
          class="relative ml-1"
          v-slot="{ open }"
          :model-value="props.modelValue.op"
          @update:model-value="(v) => emit('update:modelValue', { ...props.modelValue, op: v })"
        >
          <ListboxButton
            class="flex flex-row items-center rounded-sm border border-orange-900/[12%] px-1 py-0.5 text-left text-sm text-gray-900 hover:bg-orange-100 focus:bg-gray-100 focus:outline-none"
            :class="[open ? 'bg-orange-100' : '']"
          >
            {{ CONDITIONAL_OP_NAME[props.modelValue.op] }}
            <ChevronUpDownIcon class="mr-0.5 h-4 w-4 text-gray-500" />
          </ListboxButton>
          <FadeTransition>
            <ListboxOptions
              class="absolute left-0 top-6 z-30 w-52 origin-top-right rounded-sm bg-white px-2 py-1.5 shadow-md ring-1 ring-orange-900 ring-opacity-40"
            >
              <ListboxOption
                v-for="op in availableOps"
                :key="op"
                :value="op"
                as="div"
                class="flex flex-col p-1 hover:cursor-pointer hover:bg-orange-100 focus:bg-orange-100"
                :class="[modelValue.op === op ? 'text-orange-600' : '']"
              >
                <span> {{ CONDITIONAL_OP_NAME[op] }} </span>
              </ListboxOption>
            </ListboxOptions>
          </FadeTransition>
        </Listbox>
        <!-- Delete -->
        <button class="ml-auto text-gray-700 hover:bg-orange-100" @click="emit('update:modelValue', undefined)">
          <TrashIcon class="h-4 w-4" />
        </button>
      </div>
      <!-- Value/body -->
      <ValueInterface
        v-if="requiresScalar"
        :type="field"
        debounced
        :model-value="props.modelValue.value"
        @update:model-value="(v) => emit('update:modelValue', { ...props.modelValue, value: v })"
        class="mt-1.5 border border-orange-900/[12%] p-1 hover:bg-orange-100"
      />
    </div>
  </FadeTransition>
</template>
