import { Graph, QueryConnection, Session, Supergraph } from "@/language/core/runtime";
import { BuiltinObject } from "./object";
import { NodeReference } from "@/language/core/common/relation";
import { NodeType } from "@/language";

/** A Node is a collection of properties with an identity. */
export abstract class Node extends BuiltinObject {

	static readonly __isNode__: boolean = true;
	static readonly metatype: NodeType;

	readonly id: string;
	get parent(): Node | null {
		if (this.parentPtr === null) {
			return null;
		}
		return this._supergraph.get(this.parentPtr.id);
	}
	readonly parentPtr: NodeReference | null;
	
	// runtime
	_session: Session;
	_supergraph: Supergraph;
	_graph: Graph;
	_connection: QueryConnection | null;
	_hash: string | null;
	_ref: NodeReference | null;
	_isNew: boolean;
	_isAttached: boolean;
	_dirty: Record<string, any> | null;

	constructor(
		id: string,
		parentPtr: NodeReference | null,
		_session: Session,
		_supergraph: Supergraph,
		_graph: Graph,
		_connection: QueryConnection | null,
		_isNew: boolean,
		_isAttached: boolean,
	) {
		super(_supergraph);
		this.id = id;
		this.parentPtr = parentPtr;
		this._session = _session;
		this._supergraph = _supergraph;
		this._graph = _graph;
		this._connection = _connection;
		this._hash = this.id;
		this._ref = null;
		this._dirty = null;
		this._isNew = _isNew;
		this._isAttached = _isAttached;
	}

	get _pathKey(): string {
		throw new Error("Not implemented");
	}

	get path(): string {
		throw new Error("Not implemented");
	}

	__toRef__(): NodeReference {
		throw new Error("Not implemented");
	}

	toRef(): NodeReference {
		if (this._ref === null) {
			this._ref = this.__toRef__();
		}
		return this._ref;
	}

	/** Append a child to this Node. */
	addChild(child: Node, after?: Node, before?: Node): void {
		throw new Error("Not implemented");
	}

	/** Append multiple children to this Node. */
	addChildren(children: Node[], after?: Node, before?: Node): void {
		throw new Error("Not implemented");
	}

	/** Remove a child from this Node. */
	removeChild(child: Node): void {
		throw new Error("Not implemented");
	}

	/** Get the children of this Node. */
	getChildren(node_type?: Node): Node[] {
		throw new Error("Not implemented");
	}

	/** Get a specific child of this Node by name. */
	getChild(node_type: Node, name: string): Node | null {
		throw new Error("Not implemented");
	}

	/** Get a specific child of this Node by name, or raises an error if not found. */
	child(node_type: Node, name: string): Node {
		throw new Error("Not implemented");
	}

	/** Get the descendants of this Node. */
	getDescendants(node_type?: Node): Node[] {
		throw new Error("Not implemented");
	}
}

