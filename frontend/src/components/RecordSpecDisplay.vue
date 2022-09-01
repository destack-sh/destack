<template>
  <div
    :class="{
      'border-m rounded-l-md py-1 pl-3 text-left': true,
      'border border-l-2 ': !isWrapper,
    }"
  >
    <div class="text-sm font-medium text-gray-700" v-if="name">{{ name }}</div>
    <div class="text-sm text-gray-500" v-if="description">{{ description }}</div>
    <!-- complex type -->
    <div class="flex flex-col gap-1" v-if="childFieldSpecs.length > 0">
      <RecordSpecDisplay v-for="field in childFieldSpecs" :key="field.name" :spec="field" />
    </div>
    <!-- primitive type -->
    <div v-else-if="primitiveType?._type == 'ValueType'">
      {{ primitiveType.dtype }}
    </div>
    <div v-else-if="primitiveType?._type == 'EnumType'">enum ({{ primitiveType.values.length }})</div>
    <div v-else-if="primitiveType?._type == 'ClassLabelType'">label ({{ primitiveType.num_classes }})</div>
  </div>
</template>
<script lang="ts" setup>
import { isFieldSpec, isFieldTypePrimitive, type FieldSpec, type FieldTypePrimitive } from "@/types";
import { unravelSpec, unwrapFieldSpec } from "@/utils/spec";
import { computed, type Ref } from "vue";

const props = defineProps<{ spec: FieldSpec | FieldTypePrimitive }>();

const asSpec: Ref<FieldSpec | undefined> = computed(() => {
  if (isFieldSpec(props.spec)) {
    return props.spec as FieldSpec;
  } else {
    return undefined;
  }
});

const primitiveType: Ref<FieldTypePrimitive | undefined> = computed(() => {
  const fieldType = unwrapFieldSpec(props.spec);
  if (isFieldTypePrimitive(fieldType)) {
    return fieldType as FieldTypePrimitive;
  } else {
    return undefined;
  }
});

const childFieldSpecs = computed(() => {
  if (isFieldTypePrimitive(props.spec)) {
    return [];
  } else {
    const childSpecs = unravelSpec(props.spec);
    if (childSpecs[0] == props.spec) {
      return [];
    } else {
      return childSpecs;
    }
  }
});

const isWrapper = computed(() => primitiveType.value == null);

const name: Ref<string | undefined> = computed(() => asSpec.value?.name);
const description: Ref<string | undefined> = computed(() => asSpec.value?.description);
</script>
