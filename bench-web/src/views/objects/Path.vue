<script lang="ts" setup>
import { supergraph } from "@/globals";
import { isSourceNode } from "@/language/core/const";
import { makePath } from "@/language/core/path";
import { makeType, makeTypeConstraint } from "@/language/core/type";
import { RUN_PROPERTY_BY_FIELD_TYPE } from "@/language/runtime/process";
import {
  BenchType,
  FieldType,
  NodeReferenceData,
  NodeType,
  PathData,
  PathElementType,
  StructType,
  StructTypeOptionInfo,
  TypeKind,
  ViewData,
} from "@/proto/wire";
import { describeNode, isNode, TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { makeIcon } from "@/ui/icon";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Picker from "@/views/content/Picker.vue";
import { computed, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "valueType" | "isInput" | "isDisabled" | "isMinimal">
  >
>();
const modelValue = defineModel<PathData | undefined>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// NOTE :Incomplete: Path view just forwards to Picker for now with some wrapping :OverloadedPicker
// (obviously we want a richer Path view that supports all sorts of Paths)
const nodeValueType = computed(() =>
  makeType({
    kind: TypeKind.NODE,
    benchType: BenchType.FIELD,
    isRequired: props.valueType?.isRequired ?? false,
    constraint: makeTypeConstraint({
      nodeScopePtr: props.valueType?.constraint?.nodeScopePtr,
      nodeSubtypes: [FieldType.INPUT, FieldType.OUTPUT],
    }),
  }),
);
const nodeValue = computed(
  () => modelValue.value?.elements.findLast((e) => e.type == PathElementType.ATTRIBUTE)?.nodePtr,
);
function updateNodeValue(value: NodeReferenceData) {
  // assumes that we want the node's Runtime-field of that type :RunComputedValue
  const { node, graph } = supergraph.getLinkOrError(value);
  if (!isNode(node, NodeType.FIELD)) throw new Error(`unexpected node ${describeNode(node)}`);
  const parent = graph.getOrError(node.parentPtr!);
  if (!isSourceNode(parent)) throw new Error(`unexpected parent ${describeNode(parent)} for ${describeNode(value)}`);
  const runProperty = RUN_PROPERTY_BY_FIELD_TYPE[node.type];
  if (runProperty == null) throw new Error(`unexpected field type ${describeNode(node)}`);
  const path = makePath(parent, PathElementType.RUN, runProperty, node);
  emit("update:modelValue", path);
}

defineExpose<ViewExpose>({ self, id });
</script>
<template>
  <div>
    <Picker
      id="picker"
      :title="title ?? 'Path'"
      :icon="icon ?? makeIcon(StructTypeOptionInfo[StructType.PATH]!.icon!)"
      :value-type="nodeValueType"
      :is-input="isInput"
      :is-disabled="isDisabled"
      :is-minimal="isMinimal"
      :model-value="nodeValue"
      @update:model-value="updateNodeValue"
    />
  </div>
</template>
