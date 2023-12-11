<script lang="ts" setup>
import TypePreview from "@/components/interfaces/TypePreview.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import type { Conditional } from "@/gql/graphql";
import { renderConditional } from "@/state/database";
import type { Field } from "@/state/module";
import { computed, ref } from "vue";

const props = defineProps<{ field: Field; modelValue: Conditional }>();
const emit = defineEmits<{ (e: "update:modelValue", value?: Conditional): void }>();

const editing = ref(false);
const popoverRef = ref<HTMLDivElement | null>(null);
const popoverPin = pinAbsoluteElement(popoverRef, { pos: true, keepInView: true });

const conditionalHumanized = computed(() => renderConditional(props.modelValue));
</script>
<template>
  <button class="flex flex-row items-center rounded-xl border border-amber-900/[12%] px-1.5 py-0.5 hover:bg-amber-100">
    <!-- nocheckin: conditional -->
    <TypePreview :type="field" />
    <span class="ml-1 underline decoration-gray-300 underline-offset-4">{{ field.name }}</span
    >:
    <span class="ml-1">{{ conditionalHumanized }}</span>
  </button>
</template>
