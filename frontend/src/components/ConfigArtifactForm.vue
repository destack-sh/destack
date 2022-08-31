<template>
  <form class="flex flex-col gap-3">
    <div class="sm:col-span-4" v-for="field in props.spec" :key="field.name">
      <label for="username" class="block text-sm font-medium text-gray-700">
        {{ field.name }}
        <span v-if="isOptional(field)" class="font-normal text-gray-500">(optional)</span>
      </label>
      <div class="mt-1 flex rounded-md">
        <ArtifactSelect
          :type="field.type"
          :modelValue="artifact(modelValue[field.name])"
          @select="(artifact) => setArtifactConnection(field, artifact)"
        />
      </div>
    </div>
  </form>
</template>
<script lang="ts" setup>
import { useArtifactsStore } from "@/stores";
import { isArtifactType, type Artifact, type ArtifactConnection, type ArtifactSpec, type ArtifactType } from "@/types";
import { splitNameVersion } from "@/utils/versioning";
import ArtifactSelect from "./ArtifactSelect.vue";

const props = defineProps<{
  spec: ArtifactSpec[];
  modelValue: Record<string, ArtifactConnection>;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, ArtifactConnection>): void;
}>();

const artifactsStore = useArtifactsStore();

function artifact(connection?: ArtifactConnection) {
  if (connection == null) {
    return undefined;
  }
  const artifact = splitNameVersion(connection.dependency)[0];
  return artifactsStore.artifact(artifact);
}

function unsetArtifactConnection(field: ArtifactSpec) {
  const newRecord: Record<string, ArtifactConnection> = { ...props.modelValue };
  delete newRecord[field.name];
  return newRecord;
}

function setArtifactConnection(field: ArtifactSpec, value: Artifact) {
  if (value.latest_version == null) {
    throw new Error(`${value.name} does not have a latest version and that's all ArtifactSelect can handle`);
  }

  const newRecord: Record<string, ArtifactConnection> = { ...props.modelValue };
  newRecord[field.name] = {
    // use specific latest version if available since dependency needs to be name@version not @tag
    dependency: `${value.name}@${value.latest_version?.version}`,
    connection_name: field.name,
    connection_type: "argument",
  };
  emit("update:modelValue", newRecord);
}

function isOptional(field: ArtifactSpec): boolean | undefined {
  return isArtifactType(field.type) && (field.type as ArtifactType).optional;
}
</script>
