import { DateTime } from "@/proto";

export interface Trait {

}

export interface IsOwnable extends Trait {

}

export interface IsDeletable extends Trait {
    deletedAt: DateTime | null;
}

export interface IsArchivable extends Trait {
	archivedAt: DateTime | null;
}

export interface IsTracked extends Trait {
	createdAt: DateTime;
	createdByPtr: NodeReference | null;
	get createdBy(): IsOwnable | null;
	set createdBy(value: IsOwnable | null);
	updatedAt: DateTime;
	updatedByPtr: NodeReference | null;
	get updatedBy(): IsOwnable | null;
	set updatedBy(value: IsOwnable | null);
}