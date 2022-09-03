<template>
  <form class="mx-auto max-w-xl space-y-8 divide-y divide-gray-200 pt-8" action="">
    <div>
      <FunctionHandlerSelect label="Function" v-model="selectedFunctionHandler" />
    </div>
    <div class="grid grid-cols-1 gap-y-6 gap-x-4 pt-4 sm:grid-cols-6">
      <TextInput class="sm:col-span-4" v-model="name" label="Name" :minlength="3" :maxlength="64" />
    </div>
    <div class="pt-4" v-if="selectedFunctionHandler">
      <RecordForm
        :spec="flatMapFieldSpecs(Object.values(selectedFunctionHandler.config_spec))"
        v-model="configRecord"
      />
      <ConfigArtifactForm
        :spec="(Object.values(selectedFunctionHandler.config_spec).filter(isArtifactSpec) as any[])"
        v-model="configArtifacts"
      />
    </div>
    <div class="flex justify-end pt-4">
      <SButton type="submit" @click.prevent="submit" :text="creating ? 'Create' : 'Update'" />
    </div>
  </form>
</template>
<script lang="ts" setup>
import ConfigArtifactForm from "@/components/ConfigArtifactForm.vue";
import FunctionHandlerSelect from "@/components/FunctionHandlerSelect.vue";
import RecordForm from "@/components/RecordForm.vue";
import { useMetaStore } from "@/stores";
import {
  isArtifactSpec,
  type ArtifactConnection,
  type FlowNode,
  type FlowVersion,
  type FunctionHandlerSpec,
} from "@/types";
import { artifactConnections } from "@/utils/flows";
import { flatMapFieldSpecs } from "@/utils/spec";
import { computed, ref, toRef, watch, type Ref } from "vue";
import SButton from "./basic/SButton.vue";
import TextInput from "./basic/TextInput.vue";
const selectedFunctionHandler: Ref<FunctionHandlerSpec | null> = ref(null);

const name: Ref<string> = ref("");
const configRecord: Ref<Record<string, any>> = ref({});
const metadata: Ref<Record<string, any>> = ref({});
const configArtifacts: Ref<Record<string, ArtifactConnection>> = ref({});

const props = defineProps<{ existingNode?: FlowNode; flow: FlowVersion }>();
const emit = defineEmits<{
  (
    e: "create",
    value: {
      node: Pick<FlowNode, "name" | "function_id" | "config_arguments" | "metadata">;
      connectedArtifacts: ArtifactConnection[];
    }
  ): void;
  (e: "update", value: { node: FlowNode; connectedArtifacts: ArtifactConnection[] }): void;
}>();

const metaStore = useMetaStore();

const creating = computed(() => props.existingNode == null);
// initialize forms if not creating
watch(
  toRef(props, "existingNode"),
  () => {
    if (props.existingNode == null) {
      return;
    }
    name.value = props.existingNode.name;
    selectedFunctionHandler.value = metaStore.functionHandlersById[props.existingNode.function_id];
    configRecord.value = props.existingNode.config_arguments || {};
    configArtifacts.value = artifactConnections(props.flow, props.existingNode, "argument");
  },
  { immediate: true, deep: true }
);

function submit() {
  if (creating.value) {
    create();
  } else {
    update();
  }
}

function create() {
  const node = {
    name: name.value,
    function_id: selectedFunctionHandler.value?.id,
    config_arguments: configRecord.value,
    metadata: metadata.value,
  } as FlowNode;
  emit("create", { node, connectedArtifacts: Object.values(configArtifacts.value) });
}

function update() {
  const node = props.existingNode as FlowNode;
  const updatedNode = {
    ...node,
    name: name.value,
    function_id: selectedFunctionHandler.value?.id,
    config_arguments: configRecord.value,
    metadata: metadata.value,
  } as FlowNode;
  emit("update", { node: updatedNode, connectedArtifacts: Object.values(configArtifacts.value) });
}
</script>
