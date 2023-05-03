import { TypeTag, type RemoteObject } from "@/gql/graphql";

export const OBJECT_TYPETAGS = [TypeTag.File, TypeTag.Image, TypeTag.Audio, TypeTag.Video];

export type BasicObject = Pick<RemoteObject, "id" | "status" | "name" | "contentType" | "contentLength" | "sha512">;
