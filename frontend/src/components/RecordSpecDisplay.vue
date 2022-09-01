<template>
  <div
    :class="{
      'py-1 pl-2 text-left': true,
      'w-full rounded-md border border-gray-300 pr-2 shadow-sm': !isWrapper,
      'bg-white': !isList && !isWrapper,
    }"
  >
    <template v-if="childFieldSpecs.length > 0">
      <span class="text-sm font-medium text-gray-700" v-if="name">{{ name }}</span>
      <span class="ml-2 text-sm text-gray-500" v-if="description">{{ description }}</span>

      <!-- complex type -->
      <div class="flex flex-col gap-1">
        <RecordSpecDisplay v-for="field in childFieldSpecs" :key="field.name" :spec="field" />
      </div>
    </template>
    <template v-else>
      <!-- primitive type -->
      <div class="flex flex-row text-sm">
        <span class="flex-1 font-medium text-gray-700" v-if="name">{{ name }}</span>
        <span v-if="primitiveType?._type == 'ValueType'">
          {{ primitiveType.dtype }}
        </span>
        <span v-else-if="primitiveType?._type == 'EnumType'">enum ({{ primitiveType.values.length }})</span>
        <span v-else-if="primitiveType?._type == 'ClassLabelType'">label ({{ primitiveType.num_classes }})</span>
      </div>
      <span class="text-sm text-gray-500" v-if="description">{{ description }}</span>
    </template>
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

const isList = computed(() => isFieldSpec(props.spec) && Array.isArray((props.spec as FieldSpec).type));
// wrappers are just a level of annotation for a collection of fields
const isWrapper = computed(() => primitiveType.value == null && !isList.value);

const name: Ref<string | undefined> = computed(() => asSpec.value?.name);
const description: Ref<string | undefined> = computed(() => asSpec.value?.description);
</script>
