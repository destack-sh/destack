import { AsyncLocalStorage } from "node:async_hooks";
/**
* Thrown by a tracked read whose value is currently pending (an async memo /
* `createSignal(asyncFn)` / projection / store derivation that hasn't settled
* yet). Surfacing through the reactive graph is what suspends the consumer
* scope — the nearest enclosing `<Loading>` boundary catches the throw and
* renders its fallback until the source resolves.
*
* App code rarely catches this directly; `<Loading>` is the canonical
* handler. The error type is exposed for advanced cases — e.g. interop layers
* that bridge Solid's pending-throw protocol to a different async strategy,
* or tests that want to assert on the suspension shape.
*
* @example
* ```ts
* // Advanced: distinguish "not ready yet" from a real error in custom
* // boundary plumbing. App code should rely on `<Loading>` / `<Errored>`.
* try {
*   const value = readReactiveSource();
* } catch (err) {
*   if (err instanceof NotReadyError) throw err; // re-throw to suspend
*   reportError(err);
* }
* ```
*/
var NotReadyError = class extends Error {
	source;
	constructor(r) {
		const o = Error;
		const t = o.stackTraceLimit;
		if (t !== void 0) o.stackTraceLimit = 0;
		super();
		if (t !== void 0) o.stackTraceLimit = t;
		this.source = r;
	}
};
var NoOwnerError = class extends Error {
	constructor() {
		super("");
	}
};
var ContextNotFoundError = class extends Error {
	constructor() {
		super("");
	}
};
/**
* Brand symbol used by `Refreshable<T>` values (projection stores, async
* memos) to expose their underlying computation to `refresh()`. Not part of
* the user-facing API.
*
* @internal
*/ var $REFRESH = Symbol("refresh");
var NoHydrateContext = {
	id: Symbol("NoHydrateContext"),
	defaultValue: false
};
var sharedConfig = { getNextContextId() {
	const o = getOwner();
	if (!o) throw new Error(`getNextContextId cannot be used under non-hydrating context`);
	if (getContext(NoHydrateContext)) return void 0;
	return getNextChildId(o);
} };
var defaultSSRContext = {};
var currentOwner = null;
var OWNER_POOL_MAX = 4096;
var ownerPool = [];
function formatChildId(prefix, id) {
	const num = id.toString(36);
	const len = num.length - 1;
	return prefix + (len ? String.fromCharCode(64 + len) : "") + num;
}
function nextChildIdFor(owner, consume) {
	let counter = owner;
	while (counter._transparent && counter._parent) counter = counter._parent;
	if (counter.id != null) return formatChildId(counter.id, consume ? counter._childCount++ : counter._childCount);
	throw new Error("Cannot get child id from owner without an id");
}
function getNextChildId(owner) {
	return nextChildIdFor(owner, true);
}
var ownerCreations = 0;
function createOwner(options) {
	ownerCreations++;
	const parent = currentOwner;
	const transparent = options?.transparent ?? false;
	const id = options?.id ?? (transparent ? parent?.id : parent?.id != null ? nextChildIdFor(parent, true) : void 0);
	const ctx = parent?._context ?? defaultSSRContext;
	let owner;
	if (ownerPool.length) {
		owner = ownerPool.pop();
		owner.id = id;
		owner._transparent = transparent;
		owner._disposal = null;
		owner._parent = parent;
		owner._context = ctx;
		owner._childCount = 0;
		owner._firstChild = null;
		owner._nextSibling = null;
		owner._disposed = false;
	} else owner = {
		id,
		_transparent: transparent,
		_disposal: null,
		_parent: parent,
		_context: ctx,
		_childCount: 0,
		_firstChild: null,
		_nextSibling: null,
		_disposed: false
	};
	if (parent) {
		const lastChild = parent._firstChild;
		if (lastChild) owner._nextSibling = lastChild;
		parent._firstChild = owner;
	}
	return owner;
}
function runWithOwner(owner, fn) {
	const prev = currentOwner;
	currentOwner = owner;
	try {
		return fn();
	} finally {
		currentOwner = prev;
	}
}
function getOwner() {
	return currentOwner;
}
function onCleanup(fn) {
	const o = currentOwner;
	if (!o) return fn;
	if (!o._disposal) o._disposal = fn;
	else if (Array.isArray(o._disposal)) o._disposal.push(fn);
	else o._disposal = [o._disposal, fn];
	return fn;
}
function getContext(context, owner = currentOwner) {
	if (!owner) throw new NoOwnerError();
	const stored = owner._context[context.id];
	const value = stored !== void 0 ? stored : context.defaultValue;
	if (value === void 0) throw new ContextNotFoundError();
	return value;
}
function setContext(context, value, owner = currentOwner) {
	if (!owner) throw new NoOwnerError();
	const o = owner;
	o._context = {
		...o._context,
		[context.id]: value === void 0 ? context.defaultValue : value
	};
}
function unlinkOwner(node) {
	const parent = node._parent;
	if (!parent) return;
	if (parent._firstChild === node) parent._firstChild = node._nextSibling;
	else {
		let sibling = parent._firstChild;
		while (sibling && sibling._nextSibling !== node) sibling = sibling._nextSibling;
		if (sibling) sibling._nextSibling = node._nextSibling;
	}
	node._parent = null;
	node._nextSibling = null;
}
function disposeOwner(owner, self = true) {
	const node = owner;
	if (node._disposed) return;
	if (!node._firstChild && !node._disposal) {
		if (self) {
			node._disposed = true;
			unlinkOwner(node);
			if (ownerPool.length < OWNER_POOL_MAX) {
				node.id = void 0;
				ownerPool.push(node);
			}
		} else node._childCount = 0;
		return;
	}
	if (self) node._disposed = true;
	let child = node._firstChild;
	while (child) {
		const next = child._nextSibling;
		disposeOwner(child, true);
		child = next;
	}
	node._firstChild = null;
	node._childCount = 0;
	const d = node._disposal;
	if (d) {
		if (Array.isArray(d)) for (let i = 0, len = d.length; i < len; i++) d[i]();
		else d();
		node._disposal = null;
	}
	if (self) unlinkOwner(node);
	if (self && ownerPool.length < OWNER_POOL_MAX) {
		node.id = void 0;
		ownerPool.push(node);
	}
}
function resetOwnerForRerun(owner) {
	const node = owner;
	if (node._firstChild || node._disposal) disposeOwner(owner, false);
	node._childCount = 0;
}
function createRoot(init, options) {
	const owner = createOwner(options);
	return runWithOwner(owner, () => init(() => disposeOwner(owner)));
}
function ssrScope(fn) {
	const parent = currentOwner;
	if (!parent || parent.id == null) return fn;
	const scopeId = nextChildIdFor(parent, true);
	return () => {
		const prevId = parent.id;
		const prevCount = parent._childCount;
		parent.id = scopeId;
		parent._childCount = 0;
		try {
			let v = fn();
			while (typeof v === "function") v = v();
			return v;
		} finally {
			parent.id = prevId;
			parent._childCount = prevCount;
		}
	};
}
var LIVE_SOURCE = Symbol.for("solid.LiveSource");
var CLIENT_HOLE = /* @__PURE__ */ Object.assign(new Promise(() => {}), { $clientHole: true });
function clientHoleRead() {
	if (!sharedConfig.context?._loadingPhase) throw new Error("ssrSource: \"client\" read during SSR outside a <Loading> boundary — the server cannot run this source, so a boundary must own the position's fallback. Wrap the read in <Loading>, or declare a loadingValue/seedLoadingValue to render a provisional value instead.");
	throw new NotReadyError(CLIENT_HOLE);
}
var Observer = null;
function runWithObserver(comp, fn) {
	const prev = Observer;
	Observer = comp;
	try {
		return fn();
	} finally {
		Observer = prev;
	}
}
function createDeferredPromise() {
	let settled = false;
	let resolvePromise;
	let rejectPromise;
	const promise = new Promise((resolve, reject) => {
		resolvePromise = resolve;
		rejectPromise = reject;
	});
	return {
		promise,
		resolve(value) {
			if (settled) return;
			settled = true;
			promise.s = 1;
			promise.v = value;
			resolvePromise(value);
		},
		reject(error) {
			if (settled) return;
			settled = true;
			promise.s = 2;
			promise.v = error;
			rejectPromise(error);
		}
	};
}
function isThenable(value) {
	return value != null && typeof value === "object" && typeof value.then === "function";
}
function subscribePendingRetry(error, retry) {
	if (!(error instanceof NotReadyError)) return false;
	error.source?.then(() => retry(), () => retry());
	return true;
}
var SLOTS = /* @__PURE__ */ Symbol("settledSlots");
function settleServerAsync(initial, rerun, deferred, onSuccess, onError, isDisposed) {
	let first = true;
	const attempt = () => {
		if (isDisposed()) return;
		let current;
		try {
			current = first ? initial : rerun();
			first = false;
		} catch (error) {
			if (subscribePendingRetry(error, attempt)) return;
			onError(error);
			deferred.reject(error);
			return;
		}
		Promise.resolve(current).then((value) => {
			deferred.resolve(onSuccess(value));
		}, (error) => {
			if (subscribePendingRetry(error, attempt)) return;
			onError(error);
			deferred.reject(error);
		});
	};
	attempt();
}
var warnedServerWrites = /* @__PURE__ */ new Set();
function warnServerWrite(category) {
	if (warnedServerWrites.has(category)) return;
	warnedServerWrites.add(category);
	console.warn(category === "optimistic" ? "[SERVER_WRITE] Optimistic writes are inert on the server and will become an error. Optimistic state reverts once the async work it accompanies settles, and server output is settled state — optimistic updates only have meaning on the client." : category === "store" ? "[SERVER_WRITE] Writing a store on the server is deprecated and will become an error. Server render is pure: state changes flow from async sources (promises, async iterables), never setters — this write landed as inert data (nothing re-renders). Derive the store from its source (createStore(fn, seed)) instead of writing into it." : "[SERVER_WRITE] Writing a signal on the server is deprecated and will become an error. Server render is pure: state changes flow from async sources (promises, async iterables), never setters — this write landed as inert data (nothing re-renders). If you are bridging a subscription, make it the async source itself instead of pushing writes from its callback.");
}
function createSignal(first, second) {
	if (typeof first === "function") {
		const hasLoadingValue = second != null && "loadingValue" in second;
		return [createMemo((prev) => first(prev), second?.deferStream || second?.ssrSource || hasLoadingValue ? {
			deferStream: second?.deferStream,
			ssrSource: second?.ssrSource,
			...hasLoadingValue ? { loadingValue: second.loadingValue } : {}
		} : void 0), () => {
			warnServerWrite("signal");
		}];
	}
	return [() => first, (v) => {
		warnServerWrite("signal");
		return first = typeof v === "function" ? v(first) : v;
	}];
}
function createMemo(compute, options) {
	if (options?.sync) return createSyncMemo(compute, options);
	const ctx = sharedConfig.context;
	const owner = createOwner(options);
	const loadingState = options != null && typeof options === "object" && "loadingValue" in options ? {
		value: options.loadingValue,
		served: false
	} : void 0;
	const comp = {
		owner,
		value: void 0,
		compute,
		error: void 0,
		errored: false,
		computed: false,
		disposed: false
	};
	const creationOwner = currentOwner;
	let disposeArmed = false;
	function armDispose() {
		if (disposeArmed) return;
		disposeArmed = true;
		const o = creationOwner;
		if (!o) return;
		const flag = () => {
			comp.disposed = true;
		};
		if (!o._disposal) o._disposal = flag;
		else if (Array.isArray(o._disposal)) o._disposal.push(flag);
		else o._disposal = [o._disposal, flag];
	}
	const run = () => {
		resetOwnerForRerun(owner);
		return runWithOwner(owner, () => runWithObserver(comp, () => comp.compute(comp.value)));
	};
	function update() {
		if (comp.disposed) return;
		try {
			comp.error = void 0;
			comp.errored = false;
			const result = run();
			comp.computed = true;
			if (result !== null && typeof result === "object" && (typeof result[Symbol.asyncIterator] === "function" || typeof result.then === "function")) armDispose();
			processResult(comp, result, owner, ctx, options?.deferStream, options?.ssrSource, run, options?.serialize, loadingState);
		} catch (err) {
			if (err instanceof NotReadyError) {
				armDispose();
				subscribePendingRetry(err, update);
				if (loadingState) {
					loadingState.served = true;
					comp.value = loadingState.value;
					comp.error = void 0;
					comp.errored = false;
					comp.computed = true;
					return;
				}
			}
			comp.error = err;
			comp.errored = true;
			comp.computed = true;
		}
	}
	if (options?.ssrSource === "client") {
		comp.computed = true;
		if (loadingState) {
			loadingState.served = true;
			comp.value = loadingState.value;
		} else {
			comp.error = new NotReadyError(CLIENT_HOLE);
			comp.errored = true;
		}
	} else if (!options?.lazy) update();
	const read = () => {
		if (!comp.computed) update();
		else if (comp.sync && ctx?.commitEpoch && ctx.commitEpoch() !== comp.epoch) update();
		if (comp.errored) {
			if (comp.error?.source === CLIENT_HOLE) clientHoleRead();
			throw comp.error;
		}
		return comp.value;
	};
	read[$REFRESH] = comp;
	return read;
}
function createSyncMemo(compute, options) {
	const ctx = sharedConfig.context;
	const owner = createOwner(options);
	let value;
	let error;
	let errored = false;
	let cached = false;
	let epoch;
	function pull() {
		const prev = currentOwner;
		currentOwner = owner;
		resetOwnerForRerun(owner);
		try {
			value = compute(value);
			error = void 0;
			errored = false;
			cached = true;
			epoch = ctx?.commitEpoch?.();
			return value;
		} catch (err) {
			if (err instanceof NotReadyError) throw err;
			error = err;
			errored = true;
			cached = true;
			epoch = ctx?.commitEpoch?.();
			throw err;
		} finally {
			currentOwner = prev;
		}
	}
	if (!options?.lazy) try {
		pull();
	} catch {}
	return () => {
		if (cached && (!ctx?.commitEpoch || ctx.commitEpoch() === epoch)) {
			if (errored) throw error;
			return value;
		}
		return pull();
	};
}
function processResult(comp, result, owner, ctx, deferStream, ssrSource, rerun, serialize, loadingState) {
	if (comp.disposed) return;
	const id = owner.id;
	comp.sync = false;
	const noHydrate = serialize === false || getContext(NoHydrateContext, owner);
	if (typeof result?.[Symbol.asyncIterator] !== "function" && isThenable(result)) {
		if (result.s === 1) {
			comp.value = result.v;
			comp.error = void 0;
			comp.errored = false;
			return;
		}
		if (result.s === 2) {
			comp.error = result.v;
			comp.errored = true;
			return;
		}
		if (result.s === 3) {
			const d = result.d;
			if (loadingState) {
				loadingState.served = true;
				comp.value = loadingState.value;
			} else {
				comp.error = new NotReadyError(d.promise);
				comp.errored = true;
			}
			return;
		}
		const slot = id && ctx ? ctx[SLOTS]?.[id] : void 0;
		if (slot && slot.s) {
			result.then(void 0, () => {});
			if (slot.s === 1) {
				comp.value = slot.v;
				comp.error = void 0;
				comp.errored = false;
			} else {
				comp.error = slot.v;
				comp.errored = true;
			}
			return;
		}
		const recordSlot = (s, v, d) => {
			if (!id || !ctx) return;
			const store = ctx[SLOTS] ||= Object.create(null);
			const prev = store[id];
			if (prev) {
				prev.s = s;
				prev.v = v;
			} else store[id] = {
				s,
				v,
				d
			};
		};
		const deferred = slot ? slot.d : createDeferredPromise();
		const serializes = !!(ctx?.async && ctx.serialize && id && !noHydrate);
		if (!slot) {
			recordSlot(0, void 0, deferred);
			if (serializes) ctx.serialize(id, deferred.promise, deferStream);
		}
		const flattenResolvedIterable = (source) => {
			const hybrid = ssrSource === "hybrid" || !!source[LIVE_SOURCE] && !(!serializes && ctx?.commit && inServerComponentScope());
			result.s = 3;
			result.d = deferred;
			const iter = source[Symbol.asyncIterator]();
			return iter.next().then((r) => {
				const first = r.done ? void 0 : r.value;
				result.s = 1;
				result.v = first;
				recordSlot(1, first);
				if (!(loadingState?.served && serializes)) {
					comp.value = first;
					comp.error = void 0;
					comp.errored = false;
				}
				ctx?.commit?.();
				if (r.done) return first;
				if (hybrid) {
					closeAsyncIterator(iter);
					return first;
				}
				if (serializes) {
					let tappedFirst = true;
					return { [Symbol.asyncIterator]: () => ({
						next() {
							if (tappedFirst) {
								tappedFirst = false;
								return Promise.resolve(r);
							}
							return iter.next();
						},
						return(value) {
							return iter.return?.(value);
						}
					}) };
				}
				if (ctx?.commit && inServerComponentScope()) {
					const release = ctx.hold?.();
					const pump = () => {
						if (comp.disposed) {
							closeAsyncIterator(iter);
							release?.();
							return;
						}
						iter.next().then((nr) => {
							if (comp.disposed) {
								closeAsyncIterator(iter);
								release?.();
								return;
							}
							if (nr.done) {
								release?.();
								return;
							}
							comp.value = nr.value;
							ctx.commit();
							pump();
						}, () => release?.());
					};
					deferred.promise.then(pump, () => release?.());
				}
				return first;
			}, (error) => {
				result.s = 2;
				result.v = error;
				recordSlot(2, error);
				comp.error = error;
				comp.errored = true;
				ctx?.commit?.();
				throw error;
			});
		};
		settleServerAsync(result, () => rerun ? rerun() : result, deferred, (value) => {
			if (typeof value?.[Symbol.asyncIterator] === "function") return flattenResolvedIterable(value);
			result.s = 1;
			result.v = value;
			recordSlot(1, value);
			if (!(loadingState?.served && serializes)) {
				comp.value = value;
				comp.error = void 0;
				comp.errored = false;
			}
			ctx?.commit?.();
			return value;
		}, (error) => {
			result.s = 2;
			result.v = error;
			recordSlot(2, error);
			comp.error = error;
			comp.errored = true;
			ctx?.commit?.();
		}, () => comp.disposed);
		if (loadingState) {
			loadingState.served = true;
			comp.value = loadingState.value;
		} else {
			comp.error = new NotReadyError(deferred.promise);
			comp.errored = true;
		}
		return;
	}
	if (typeof result?.[Symbol.asyncIterator] === "function") {
		const serializes = !!(ctx?.async && ctx.serialize && id && !noHydrate);
		if (ssrSource === "hybrid" || !!result[LIVE_SOURCE] && !(!serializes && ctx?.commit && inServerComponentScope())) {
			let currentResult = result;
			let iter;
			const deferred = createDeferredPromise();
			const runFirst = () => {
				const source = currentResult ?? (rerun ? rerun() : result);
				currentResult = void 0;
				const nextIterator = source?.[Symbol.asyncIterator];
				if (typeof nextIterator !== "function") throw new Error("Expected async iterator while retrying server createMemo");
				iter = nextIterator.call(source);
				return iter.next().then((value) => {
					if (!value.done) closeAsyncIterator(iter);
					return value.value;
				});
			};
			settleServerAsync(runFirst(), runFirst, deferred, (value) => {
				if (!(loadingState?.served && serializes)) {
					comp.value = value;
					comp.error = void 0;
					comp.errored = false;
				}
				ctx?.commit?.();
				return value;
			}, (error) => {
				comp.error = error;
				comp.errored = true;
				ctx?.commit?.();
			}, () => comp.disposed);
			if (serializes) ctx.serialize(id, deferred.promise, deferStream);
			if (loadingState) {
				loadingState.served = true;
				comp.value = loadingState.value;
			} else {
				comp.error = new NotReadyError(deferred.promise);
				comp.errored = true;
			}
		} else {
			let currentResult = result;
			let iter;
			let firstResult;
			const deferred = createDeferredPromise();
			const runFirst = () => {
				const source = currentResult ?? (rerun ? rerun() : result);
				currentResult = void 0;
				const nextIterator = source?.[Symbol.asyncIterator];
				if (typeof nextIterator !== "function") throw new Error("Expected async iterator while retrying server createMemo");
				iter = nextIterator.call(source);
				return iter.next().then((value) => {
					firstResult = value;
					return Promise.resolve();
				});
			};
			settleServerAsync(runFirst(), runFirst, deferred, () => {
				const resolved = firstResult;
				if (resolved && !resolved.done && !(loadingState?.served && serializes)) comp.value = resolved.value;
				if (!(loadingState?.served && serializes)) {
					comp.error = void 0;
					comp.errored = false;
				}
				ctx?.commit?.();
			}, (error) => {
				comp.error = error;
				comp.errored = true;
				ctx?.commit?.();
			}, () => comp.disposed);
			if (serializes) {
				let tappedFirst = true;
				const tapped = { [Symbol.asyncIterator]: () => ({
					next() {
						if (tappedFirst) {
							tappedFirst = false;
							return deferred.promise.then(() => firstResult?.done ? {
								done: true,
								value: void 0
							} : firstResult);
						}
						return iter.next().then((r) => r);
					},
					return(value) {
						return iter.return?.(value);
					}
				}) };
				ctx.serialize(id, tapped, deferStream);
			} else if (ctx?.commit && inServerComponentScope()) {
				const release = ctx.hold?.();
				const pump = () => {
					if (comp.disposed) {
						closeAsyncIterator(iter);
						release?.();
						return;
					}
					iter.next().then((r) => {
						if (comp.disposed) {
							closeAsyncIterator(iter);
							release?.();
							return;
						}
						if (r.done) {
							release?.();
							return;
						}
						comp.value = r.value;
						ctx.commit();
						pump();
					}, () => release?.());
				};
				deferred.promise.then(pump, () => release?.());
			}
			if (loadingState) {
				loadingState.served = true;
				comp.value = loadingState.value;
			} else {
				comp.error = new NotReadyError(deferred.promise);
				comp.errored = true;
			}
		}
		return;
	}
	if (loadingState?.served) {
		if (ctx?.async && ctx.serialize && id && !noHydrate) ctx.serialize(id, Promise.resolve(result), deferStream);
		return;
	}
	comp.value = result;
	comp.sync = true;
	comp.epoch = ctx?.commitEpoch?.();
}
function closeAsyncIterator(iter, value) {
	const returned = iter.return?.(value);
	if (returned && typeof returned.then === "function") returned.then(void 0, () => {});
}
var projectionTraces = /* @__PURE__ */ new WeakMap();
function getProjectionTrace(value) {
	return typeof value === "object" && value !== null ? projectionTraces.get(value) : void 0;
}
var ErrorContext = {
	id: Symbol("ErrorContext"),
	defaultValue: null
};
var RevealGroupContext = {
	id: Symbol("RevealGroupContext"),
	defaultValue: null
};
function runWithBoundaryErrorContext(owner, render, onError, context, boundaryId) {
	const prevCtx = sharedConfig.context;
	const prevBoundary = context?._currentBoundaryId;
	const prevLoadingPhase = context?._loadingPhase;
	if (context) {
		sharedConfig.context = context;
		if (boundaryId !== void 0) {
			context._currentBoundaryId = boundaryId;
			context._loadingPhase = true;
		}
	}
	try {
		return runWithOwner(owner, () => {
			const parentHandler = getContext(ErrorContext);
			setContext(ErrorContext, (err) => onError(err, parentHandler));
			return render();
		});
	} finally {
		if (context) {
			if (boundaryId !== void 0) {
				context._currentBoundaryId = prevBoundary;
				context._loadingPhase = prevLoadingPhase;
			}
			sharedConfig.context = prevCtx;
		}
	}
}
function createErrorBoundary(fn, fallback) {
	const ctx = sharedConfig.context;
	const parent = getOwner();
	const owner = createOwner();
	setContext(RevealGroupContext, null, owner);
	const outputOwner = ctx ? createOwner() : void 0;
	let pending;
	const resolve = () => {
		const resolved = pending ? ctx.ssr(pending.t, ...pending.h) : ctx.resolve(runWithOwner(createOwner(), fn));
		pending = resolved?.p?.length ? resolved : void 0;
		if (pending) {
			const all = Promise.all(pending.p);
			if (pending.p.some((p) => p.$clientHole)) all.$clientHole = true;
			throw new NotReadyError(all);
		}
		return resolved;
	};
	const renderFallback = (err) => ctx ? runWithOwner(parent, () => {
		return runWithOwner(outputOwner, () => fallback(() => err, () => {}));
	}) : fallback(() => err, () => {});
	const serializeError = (err) => {
		if (ctx && owner.id && !runWithOwner(owner, () => getContext(NoHydrateContext))) ctx.serialize(owner.id, err);
	};
	const handleError = (err) => {
		serializeError(err);
		return renderFallback(err);
	};
	return Object.assign(() => {
		let result;
		let handled = false;
		if (ctx && !pending) disposeOwner(owner, false);
		try {
			result = ctx ? runWithBoundaryErrorContext(owner, resolve, (err) => {
				if (err instanceof NotReadyError) throw err;
				handled = true;
				result = handleError(err);
				throw err;
			}) : runWithOwner(owner, fn);
		} catch (err) {
			if (err instanceof NotReadyError) throw err;
			pending = void 0;
			result = handled ? result : handleError(err);
		}
		return result;
	}, { $lhSkip: true });
}
var ServerComponentContext = { id: Symbol("ServerComponentContext") };
function inServerComponentScope() {
	const o = currentOwner;
	return !!o && o._context[ServerComponentContext.id] === true;
}
function onSettled(callback) {
	const o = getOwner();
	if (o?.id != null) getNextChildId(o);
}
function ssrHandleError(err, probe) {
	if (err instanceof NotReadyError) return err.source;
	if (probe) return;
	const handler = getOwner() ? getContext(ErrorContext) : null;
	if (handler) {
		handler(err);
		return;
	}
	throw err;
}
function Errored(props) {
	return createErrorBoundary(() => props.children, (err, reset) => {
		const f = props.fallback;
		return typeof f === "function" && f.length ? f(err, reset) : f;
	});
}
/**
* References
* - https://compat-table.github.io/compat-table/es6/
* - MDN
*/
var Feature = /* @__PURE__ */ function(Feature) {
	Feature[Feature["AggregateError"] = 1] = "AggregateError";
	Feature[Feature["ArrowFunction"] = 2] = "ArrowFunction";
	Feature[Feature["ErrorPrototypeStack"] = 4] = "ErrorPrototypeStack";
	Feature[Feature["ObjectAssign"] = 8] = "ObjectAssign";
	Feature[Feature["BigIntTypedArray"] = 16] = "BigIntTypedArray";
	Feature[Feature["RegExp"] = 32] = "RegExp";
	Feature[Feature["Temporal"] = 64] = "Temporal";
	return Feature;
}({});
var SYM_ASYNC_ITERATOR = Symbol.asyncIterator;
var SYM_HAS_INSTANCE = Symbol.hasInstance;
var SYM_IS_CONCAT_SPREADABLE = Symbol.isConcatSpreadable;
var SYM_ITERATOR = Symbol.iterator;
var SYM_MATCH = Symbol.match;
var SYM_MATCH_ALL = Symbol.matchAll;
var SYM_REPLACE = Symbol.replace;
var SYM_SEARCH = Symbol.search;
var SYM_SPECIES = Symbol.species;
var SYM_SPLIT = Symbol.split;
var SYM_TO_PRIMITIVE = Symbol.toPrimitive;
var SYM_TO_STRING_TAG = Symbol.toStringTag;
var SYM_UNSCOPABLES = Symbol.unscopables;
var SYMBOL_STRING = {
	[0]: "Symbol.asyncIterator",
	[1]: "Symbol.hasInstance",
	[2]: "Symbol.isConcatSpreadable",
	[3]: "Symbol.iterator",
	[4]: "Symbol.match",
	[5]: "Symbol.matchAll",
	[6]: "Symbol.replace",
	[7]: "Symbol.search",
	[8]: "Symbol.species",
	[9]: "Symbol.split",
	[10]: "Symbol.toPrimitive",
	[11]: "Symbol.toStringTag",
	[12]: "Symbol.unscopables"
};
var INV_SYMBOL_REF = {
	[SYM_ASYNC_ITERATOR]: 0,
	[SYM_HAS_INSTANCE]: 1,
	[SYM_IS_CONCAT_SPREADABLE]: 2,
	[SYM_ITERATOR]: 3,
	[SYM_MATCH]: 4,
	[SYM_MATCH_ALL]: 5,
	[SYM_REPLACE]: 6,
	[SYM_SEARCH]: 7,
	[SYM_SPECIES]: 8,
	[SYM_SPLIT]: 9,
	[SYM_TO_PRIMITIVE]: 10,
	[SYM_TO_STRING_TAG]: 11,
	[SYM_UNSCOPABLES]: 12
};
var CONSTANT_STRING = {
	[2]: "!0",
	[3]: "!1",
	[1]: "void 0",
	[0]: "null",
	[4]: "-0",
	[5]: "1/0",
	[6]: "-1/0",
	[7]: "0/0"
};
var ERROR_CONSTRUCTOR_STRING = {
	[0]: "Error",
	[1]: "EvalError",
	[2]: "RangeError",
	[3]: "ReferenceError",
	[4]: "SyntaxError",
	[5]: "TypeError",
	[6]: "URIError"
};
function createSerovalNode(t, i, s, c, m, p, e, a, f, b, o, l) {
	return {
		t,
		i,
		s,
		c,
		m,
		p,
		e,
		a,
		f,
		b,
		o,
		l
	};
}
function createConstantNode(value) {
	return createSerovalNode(2, void 0, value, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
var TRUE_NODE = /* @__PURE__ */ createConstantNode(2);
var FALSE_NODE = /* @__PURE__ */ createConstantNode(3);
var UNDEFINED_NODE = /* @__PURE__ */ createConstantNode(1);
var NULL_NODE = /* @__PURE__ */ createConstantNode(0);
var NEG_ZERO_NODE = /* @__PURE__ */ createConstantNode(4);
var INFINITY_NODE = /* @__PURE__ */ createConstantNode(5);
var NEG_INFINITY_NODE = /* @__PURE__ */ createConstantNode(6);
var NAN_NODE = /* @__PURE__ */ createConstantNode(7);
var MIN_JSON_STRINGIFY_LENGTH = 64;
var JSON_ESCAPE_DIFFERENCES = /[\x00-\x07\x0b\x0e-\x1f<\u2028\u2029\ud800-\udfff]/;
function serializeChar(str) {
	switch (str) {
		case "\"": return "\\\"";
		case "\\": return "\\\\";
		case "\n": return "\\n";
		case "\r": return "\\r";
		case "\b": return "\\b";
		case "	": return "\\t";
		case "\f": return "\\f";
		case "<": return "\\x3C";
		case "\u2028": return "\\u2028";
		case "\u2029": return "\\u2029";
		default: return;
	}
}
function serializeString(str) {
	if (str.length >= MIN_JSON_STRINGIFY_LENGTH && !JSON_ESCAPE_DIFFERENCES.test(str)) return JSON.stringify(str).slice(1, -1);
	let result = "";
	let lastPos = 0;
	let replacement;
	for (let i = 0, len = str.length; i < len; i++) {
		replacement = serializeChar(str[i]);
		if (replacement) {
			result += str.slice(lastPos, i) + replacement;
			lastPos = i + 1;
		}
	}
	if (lastPos === 0) result = str;
	else result += str.slice(lastPos);
	return result;
}
function deserializeReplacer(str) {
	switch (str) {
		case "\\\\": return "\\";
		case "\\\"": return "\"";
		case "\\n": return "\n";
		case "\\r": return "\r";
		case "\\b": return "\b";
		case "\\t": return "	";
		case "\\f": return "\f";
		case "\\x3C": return "<";
		case "\\u2028": return "\u2028";
		case "\\u2029": return "\u2029";
		default: return str;
	}
}
function deserializeString(str) {
	if (typeof str === "string" && !str.includes("\\")) return str;
	return str.replace(/(\\\\|\\"|\\n|\\r|\\b|\\t|\\f|\\u2028|\\u2029|\\x3C)/g, deserializeReplacer);
}
var { toString: objectToString } = Object.prototype;
var STEP_ERROR_CODES = {
	parsing: 1,
	serialization: 2,
	deserialization: 3
};
function getErrorMessageProd(type) {
	return `Seroval Error (step: ${STEP_ERROR_CODES[type]})`;
}
var getErrorMessage = (type, cause) => getErrorMessageProd(type);
var SerovalError = class extends Error {
	constructor(type, cause) {
		super(getErrorMessage(type, cause));
		this.cause = cause;
	}
};
var SerovalParserError = class extends SerovalError {
	constructor(cause) {
		super("parsing", cause);
	}
};
function getSpecificErrorMessage(code) {
	return `Seroval Error (specific: ${code})`;
}
var SerovalUnsupportedTypeError = class extends Error {
	constructor(value) {
		super(getSpecificErrorMessage(1));
		this.value = value;
	}
};
var SerovalUnsupportedNodeError = class extends Error {
	constructor(node) {
		super(getSpecificErrorMessage(2));
	}
};
var SerovalMissingPluginError = class extends Error {
	constructor(tag) {
		super(getSpecificErrorMessage(3));
	}
};
var SerovalMissingReferenceError = class extends Error {
	constructor(value) {
		super(getSpecificErrorMessage(5));
		this.value = value;
	}
};
var SerovalDepthLimitError = class extends Error {
	constructor(limit) {
		super(getSpecificErrorMessage(9));
	}
};
var REFERENCES_KEY = "__SEROVAL_REFS__";
var GLOBAL_CONTEXT_R = `self.\$R`;
function getCrossReferenceHeader(id) {
	if (id == null) return `${GLOBAL_CONTEXT_R}=${GLOBAL_CONTEXT_R}||[]`;
	return `(${GLOBAL_CONTEXT_R}=${GLOBAL_CONTEXT_R}||{})["${serializeString(id)}"]=[]`;
}
var REFERENCE = /* @__PURE__ */ new Map();
var INV_REFERENCE = /* @__PURE__ */ new Map();
function hasReferenceID(value) {
	return REFERENCE.has(value);
}
function getReferenceID(value) {
	if (hasReferenceID(value)) return REFERENCE.get(value);
	throw new SerovalMissingReferenceError(value);
}
if (typeof globalThis !== "undefined") Object.defineProperty(globalThis, REFERENCES_KEY, {
	value: INV_REFERENCE,
	configurable: true,
	writable: false,
	enumerable: false
});
else if (typeof window !== "undefined") Object.defineProperty(window, REFERENCES_KEY, {
	value: INV_REFERENCE,
	configurable: true,
	writable: false,
	enumerable: false
});
else if (typeof self !== "undefined") Object.defineProperty(self, REFERENCES_KEY, {
	value: INV_REFERENCE,
	configurable: true,
	writable: false,
	enumerable: false
});
else if (typeof global !== "undefined") Object.defineProperty(global, REFERENCES_KEY, {
	value: INV_REFERENCE,
	configurable: true,
	writable: false,
	enumerable: false
});
function getErrorConstructor(error) {
	if (error instanceof EvalError) return 1;
	if (error instanceof RangeError) return 2;
	if (error instanceof ReferenceError) return 3;
	if (error instanceof SyntaxError) return 4;
	if (error instanceof TypeError) return 5;
	if (error instanceof URIError) return 6;
	return 0;
}
function getInitialErrorOptions(error) {
	const construct = ERROR_CONSTRUCTOR_STRING[getErrorConstructor(error)];
	if (error.name !== construct) return { name: error.name };
	if (error.constructor.name !== construct) return { name: error.constructor.name };
	return {};
}
function getErrorOptions(error, features) {
	let options = getInitialErrorOptions(error);
	const names = Object.getOwnPropertyNames(error);
	for (let i = 0, len = names.length, name; i < len; i++) {
		name = names[i];
		if (name !== "name" && name !== "message") {
			if (name === "stack") {
				if (features & 4) {
					options = options || {};
					options[name] = error[name];
				}
			} else {
				options = options || {};
				options[name] = error[name];
			}
		}
	}
	return options;
}
function getObjectFlag(obj) {
	if (Object.isFrozen(obj)) return 3;
	if (Object.isSealed(obj)) return 2;
	if (Object.isExtensible(obj)) return 0;
	return 1;
}
function createNumberNode(value) {
	switch (value) {
		case Number.POSITIVE_INFINITY: return INFINITY_NODE;
		case Number.NEGATIVE_INFINITY: return NEG_INFINITY_NODE;
	}
	if (value !== value) return NAN_NODE;
	if (Object.is(value, -0)) return NEG_ZERO_NODE;
	return createSerovalNode(0, void 0, value, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createStringNode(value) {
	return createSerovalNode(1, void 0, serializeString(value), void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createBigIntNode(current) {
	return createSerovalNode(3, void 0, "" + current, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createIndexedValueNode(id) {
	return createSerovalNode(4, id, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createDateNode(id, current) {
	const timestamp = current.valueOf();
	return createSerovalNode(5, id, timestamp !== timestamp ? "" : current.toISOString(), void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createTemporalNode(id, type, current) {
	return createSerovalNode(36, id, current.toString(), type, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createRegExpNode(id, current) {
	return createSerovalNode(6, id, void 0, serializeString(current.source), current.flags, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createWKSymbolNode(id, current) {
	return createSerovalNode(17, id, INV_SYMBOL_REF[current], void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createReferenceNode(id, ref) {
	return createSerovalNode(18, id, serializeString(getReferenceID(ref)), void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createPluginNode(id, tag, value) {
	return createSerovalNode(25, id, value, serializeString(tag), void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createArrayNode(id, current, parsedItems) {
	return createSerovalNode(9, id, void 0, void 0, void 0, void 0, void 0, parsedItems, void 0, void 0, getObjectFlag(current), void 0);
}
function createBoxedNode(id, boxed) {
	return createSerovalNode(21, id, void 0, void 0, void 0, void 0, void 0, void 0, boxed, void 0, void 0, void 0);
}
var MAX_TYPED_ARRAY_LENGTH = 1e6;
function createTypedArrayNode(id, current, buffer) {
	if (current.length > MAX_TYPED_ARRAY_LENGTH) throw new SerovalUnsupportedTypeError(current);
	return createSerovalNode(15, id, void 0, current.constructor.name, void 0, void 0, void 0, void 0, buffer, current.byteOffset, void 0, current.length);
}
function createBigIntTypedArrayNode(id, current, buffer) {
	if (current.length > MAX_TYPED_ARRAY_LENGTH) throw new SerovalUnsupportedTypeError(current);
	return createSerovalNode(16, id, void 0, current.constructor.name, void 0, void 0, void 0, void 0, buffer, current.byteOffset, void 0, current.length);
}
function createDataViewNode(id, current, buffer) {
	if (current.byteLength > MAX_TYPED_ARRAY_LENGTH) throw new SerovalUnsupportedTypeError(current);
	return createSerovalNode(20, id, void 0, void 0, void 0, void 0, void 0, void 0, buffer, current.byteOffset, void 0, current.byteLength);
}
function createErrorNode(id, current, options) {
	return createSerovalNode(13, id, getErrorConstructor(current), void 0, serializeString(current.message), options, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createAggregateErrorNode(id, current, options) {
	return createSerovalNode(14, id, getErrorConstructor(current), void 0, serializeString(current.message), options, void 0, void 0, void 0, void 0, void 0, void 0);
}
function createSetNode(id, items) {
	return createSerovalNode(7, id, void 0, void 0, void 0, void 0, void 0, items, void 0, void 0, void 0, void 0);
}
function createIteratorFactoryInstanceNode(factory, items) {
	return createSerovalNode(28, void 0, void 0, void 0, void 0, void 0, void 0, [factory, items], void 0, void 0, void 0, void 0);
}
function createAsyncIteratorFactoryInstanceNode(factory, items) {
	return createSerovalNode(30, void 0, void 0, void 0, void 0, void 0, void 0, [factory, items], void 0, void 0, void 0, void 0);
}
function createStreamConstructorNode(id, factory, sequence) {
	return createSerovalNode(31, id, void 0, void 0, void 0, void 0, void 0, sequence, factory, void 0, void 0, void 0);
}
function createStreamNextNode(id, parsed) {
	return createSerovalNode(32, id, void 0, void 0, void 0, void 0, void 0, void 0, parsed, void 0, void 0, void 0);
}
function createStreamThrowNode(id, parsed) {
	return createSerovalNode(33, id, void 0, void 0, void 0, void 0, void 0, void 0, parsed, void 0, void 0, void 0);
}
function createStreamReturnNode(id, parsed) {
	return createSerovalNode(34, id, void 0, void 0, void 0, void 0, void 0, void 0, parsed, void 0, void 0, void 0);
}
function createSequenceNode(id, sequence, throwAt, doneAt) {
	return createSerovalNode(35, id, throwAt, void 0, void 0, void 0, void 0, sequence, void 0, void 0, void 0, doneAt);
}
/**
* An opaque reference allows hiding values from the serializer.
*/
var OpaqueReference = class {
	constructor(value, replacement) {
		this.value = value;
		this.replacement = replacement;
	}
};
var PROMISE_CONSTRUCTOR = () => {
	const resolver = {
		p: 0,
		s: 0,
		f: 0
	};
	resolver.p = new Promise((resolve, reject) => {
		resolver.s = resolve;
		resolver.f = reject;
	});
	return resolver;
};
var PROMISE_SUCCESS = (resolver, data) => {
	resolver.s(data);
	resolver.p.s = 1;
	resolver.p.v = data;
};
var PROMISE_FAILURE = (resolver, data) => {
	resolver.f(data);
	resolver.p.s = 2;
	resolver.p.v = data;
};
var SERIALIZED_PROMISE_CONSTRUCTOR = /* @__PURE__ */ PROMISE_CONSTRUCTOR.toString();
var SERIALIZED_PROMISE_SUCCESS = /* @__PURE__ */ PROMISE_SUCCESS.toString();
var SERIALIZED_PROMISE_FAILURE = /* @__PURE__ */ PROMISE_FAILURE.toString();
var STREAM_CONSTRUCTOR = () => {
	const buffer = [];
	const listeners = [];
	let alive = true;
	let success = false;
	let count = 0;
	const internal = {
		flush(value, mode, x) {
			for (x = 0; x < count; x++) {
				const listener = listeners[x];
				if (listener) listener[mode](value);
			}
		},
		up(listener, x, z, current) {
			for (x = 0, z = buffer.length; x < z; x++) {
				current = buffer[x];
				if (!alive && x === z - 1) listener[success ? "return" : "throw"](current);
				else listener.next(current);
			}
		},
		on(listener, temp = 0) {
			let subscribed = alive;
			if (alive) {
				for (temp = 0; temp < count; temp++) if (!listeners[temp]) break;
				if (temp === count) count++;
				listeners[temp] = listener;
			}
			internal.up(listener);
			return () => {
				if (alive && subscribed) {
					subscribed = false;
					listeners[temp] = void 0;
					while (count > 0 && !listeners[count - 1]) count--;
					listeners.length = count;
				}
			};
		}
	};
	return {
		__SEROVAL_STREAM__: true,
		on(listener) {
			return internal.on(listener);
		},
		next(value) {
			if (alive) {
				buffer.push(value);
				internal.flush(value, "next");
			}
		},
		throw(value) {
			if (alive) {
				buffer.push(value);
				internal.flush(value, "throw");
				alive = false;
				success = false;
				listeners.length = 0;
			}
		},
		return(value) {
			if (alive) {
				buffer.push(value);
				internal.flush(value, "return");
				alive = false;
				success = true;
				listeners.length = 0;
			}
		}
	};
};
var SERIALIZED_STREAM_CONSTRUCTOR = /* @__PURE__ */ STREAM_CONSTRUCTOR.toString();
var ITERATOR_CONSTRUCTOR = (symbol) => (sequence) => () => {
	let index = 0;
	const instance = {
		[symbol]() {
			return instance;
		},
		next() {
			if (index > sequence.d) return {
				done: true,
				value: void 0
			};
			const currentIndex = index++;
			const data = sequence.v[currentIndex];
			if (currentIndex === sequence.t) throw data;
			return {
				done: currentIndex === sequence.d,
				value: data
			};
		}
	};
	return instance;
};
var SERIALIZED_ITERATOR_CONSTRUCTOR = /* @__PURE__ */ ITERATOR_CONSTRUCTOR.toString();
var ASYNC_ITERATOR_CONSTRUCTOR = (symbol, createPromise) => (stream) => () => {
	let count = 0;
	let doneAt = -1;
	let isThrow = false;
	const buffer = [];
	const pending = [];
	const internal = { finalize(i = 0, len = pending.length) {
		for (; i < len; i++) pending[i].s({
			done: true,
			value: void 0
		});
	} };
	stream.on({
		next(value) {
			const temp = pending.shift();
			if (temp) temp.s({
				done: false,
				value
			});
			buffer.push(value);
		},
		throw(value) {
			const temp = pending.shift();
			if (temp) temp.f(value);
			internal.finalize();
			doneAt = buffer.length;
			isThrow = true;
			buffer.push(value);
		},
		return(value) {
			const temp = pending.shift();
			if (temp) temp.s({
				done: true,
				value
			});
			internal.finalize();
			doneAt = buffer.length;
			buffer.push(value);
		}
	});
	const instance = {
		[symbol]() {
			return instance;
		},
		next() {
			if (doneAt === -1) {
				const index = count++;
				if (index >= buffer.length) {
					const temp = createPromise();
					pending.push(temp);
					return temp.p;
				}
				return {
					done: false,
					value: buffer[index]
				};
			}
			if (count > doneAt) return {
				done: true,
				value: void 0
			};
			const index = count++;
			const value = buffer[index];
			if (index !== doneAt) return {
				done: false,
				value
			};
			if (isThrow) throw value;
			return {
				done: true,
				value
			};
		}
	};
	return instance;
};
var SERIALIZED_ASYNC_ITERATOR_CONSTRUCTOR = /* @__PURE__ */ ASYNC_ITERATOR_CONSTRUCTOR.toString();
var ARRAY_BUFFER_CONSTRUCTOR = (b64) => {
	const decoded = atob(b64);
	const length = decoded.length;
	const arr = new Uint8Array(length);
	for (let i = 0; i < length; i++) arr[i] = decoded.charCodeAt(i);
	return arr.buffer;
};
var SERIALIZED_ARRAY_BUFFER_CONSTRUCTOR = /* @__PURE__ */ ARRAY_BUFFER_CONSTRUCTOR.toString();
function isSequence(value) {
	return "__SEROVAL_SEQUENCE__" in value;
}
function createSequence(values, throwAt, doneAt) {
	return {
		__SEROVAL_SEQUENCE__: true,
		v: values,
		t: throwAt,
		d: doneAt
	};
}
function createSequenceFromIterable(source) {
	const values = [];
	let throwsAt = -1;
	let doneAt = -1;
	const iterator = source[SYM_ITERATOR]();
	while (true) try {
		const value = iterator.next();
		values.push(value.value);
		if (value.done) {
			doneAt = values.length - 1;
			break;
		}
	} catch (error) {
		throwsAt = values.length;
		values.push(error);
	}
	return createSequence(values, throwsAt, doneAt);
}
var ITERATOR = {};
var ASYNC_ITERATOR = {};
/**
* Placeholder references
*/
var SPECIAL_REFS = {
	[0]: {},
	[1]: {},
	[2]: {},
	[3]: {},
	[4]: {},
	[5]: {}
};
var SPECIAL_REF_STRING = {
	[0]: "[]",
	[1]: SERIALIZED_PROMISE_CONSTRUCTOR,
	[2]: SERIALIZED_PROMISE_SUCCESS,
	[3]: SERIALIZED_PROMISE_FAILURE,
	[4]: SERIALIZED_STREAM_CONSTRUCTOR,
	[5]: SERIALIZED_ARRAY_BUFFER_CONSTRUCTOR
};
function isStream(value) {
	return "__SEROVAL_STREAM__" in value;
}
function createStream() {
	return STREAM_CONSTRUCTOR();
}
function createStreamFromAsyncIterable(iterable, cleanups) {
	const stream = createStream();
	const iterator = iterable[SYM_ASYNC_ITERATOR]();
	let cancelled = false;
	let done = false;
	cleanups === null || cleanups === void 0 || cleanups.push(() => {
		if (!(done || cancelled)) {
			cancelled = true;
			Promise.resolve().then(() => {
				var _iterator$return;
				return (_iterator$return = iterator.return) === null || _iterator$return === void 0 ? void 0 : _iterator$return.call(iterator);
			}).catch(() => {});
		}
	});
	async function push() {
		try {
			while (!cancelled) {
				const value = await iterator.next();
				if (cancelled) return;
				if (value.done) {
					done = true;
					stream.return(value.value);
					break;
				}
				stream.next(value.value);
			}
		} catch (error) {
			done = true;
			if (!cancelled) stream.throw(error);
		}
	}
	push().catch(() => {});
	return stream;
}
function createBaseParserContext(mode, options) {
	var _options$compactArray;
	return {
		plugins: options.plugins,
		mode,
		marked: /* @__PURE__ */ new Set(),
		features: 127 ^ (options.disabledFeatures || 0),
		refs: options.refs || /* @__PURE__ */ new Map(),
		depthLimit: options.depthLimit || 1e3,
		compactArrayBufferViews: (_options$compactArray = options.compactArrayBufferViews) !== null && _options$compactArray !== void 0 ? _options$compactArray : false
	};
}
/**
* Ensures that the value (based on an identifier) has been visited by the parser.
* @param ctx
* @param id
*/
function markParserRef(ctx, id) {
	ctx.marked.add(id);
}
/**
* Creates an identifier for a value
* @param ctx
* @param current
*/
function createIndexForValue(ctx, current) {
	const id = ctx.refs.size;
	ctx.refs.set(current, id);
	return id;
}
function getNodeForIndexedValue(ctx, current) {
	const registeredId = ctx.refs.get(current);
	if (registeredId != null) {
		markParserRef(ctx, registeredId);
		return {
			type: 1,
			value: createIndexedValueNode(registeredId)
		};
	}
	return {
		type: 0,
		value: createIndexForValue(ctx, current)
	};
}
function getReferenceNode(ctx, current) {
	const indexed = getNodeForIndexedValue(ctx, current);
	if (indexed.type === 1) return indexed;
	if (hasReferenceID(current)) return {
		type: 2,
		value: createReferenceNode(indexed.value, current)
	};
	return indexed;
}
/**
* Parsing methods
*/
function parseWellKnownSymbol(ctx, current) {
	const ref = getReferenceNode(ctx, current);
	if (ref.type !== 0) return ref.value;
	if (current in INV_SYMBOL_REF) return createWKSymbolNode(ref.value, current);
	throw new SerovalUnsupportedTypeError(current);
}
function parseSpecialReference(ctx, ref) {
	const result = getNodeForIndexedValue(ctx, SPECIAL_REFS[ref]);
	if (result.type === 1) return result.value;
	return createSerovalNode(26, result.value, ref, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0, void 0);
}
function parseIteratorFactory(ctx) {
	const result = getNodeForIndexedValue(ctx, ITERATOR);
	if (result.type === 1) return result.value;
	return createSerovalNode(27, result.value, void 0, void 0, void 0, void 0, void 0, void 0, parseWellKnownSymbol(ctx, SYM_ITERATOR), void 0, void 0, void 0);
}
function parseAsyncIteratorFactory(ctx) {
	const result = getNodeForIndexedValue(ctx, ASYNC_ITERATOR);
	if (result.type === 1) return result.value;
	return createSerovalNode(29, result.value, void 0, void 0, void 0, void 0, void 0, [parseSpecialReference(ctx, 1), parseWellKnownSymbol(ctx, SYM_ASYNC_ITERATOR)], void 0, void 0, void 0, void 0);
}
function createObjectNode(id, current, empty, record) {
	return createSerovalNode(empty ? 11 : 10, id, void 0, void 0, void 0, record, void 0, void 0, void 0, void 0, getObjectFlag(current), void 0);
}
function createMapNode(ctx, id, k, v) {
	return createSerovalNode(8, id, void 0, void 0, void 0, void 0, {
		k,
		v
	}, void 0, parseSpecialReference(ctx, 0), void 0, void 0, void 0);
}
function createPromiseConstructorNode(ctx, id, resolver) {
	return createSerovalNode(22, id, resolver, void 0, void 0, void 0, void 0, void 0, parseSpecialReference(ctx, 1), void 0, void 0, void 0);
}
function getArrayBufferView(ctx, current) {
	if (!ctx.compactArrayBufferViews) return current;
	const buffer = new Uint8Array(current.buffer, current.byteOffset, current.byteLength).slice().buffer;
	const Constructor = current.constructor;
	return new Constructor(buffer);
}
function encodeArrayBuffer(current) {
	if (typeof Buffer !== "undefined") return Buffer.from(current).toString("base64");
	const bytes = new Uint8Array(current);
	if (typeof bytes.toBase64 === "function") return bytes.toBase64();
	let result = "";
	for (let i = 0, len = bytes.length; i < len; i++) result += String.fromCharCode(bytes[i]);
	return btoa(result);
}
function createArrayBufferNode(ctx, id, current) {
	return createSerovalNode(19, id, encodeArrayBuffer(current), void 0, void 0, void 0, void 0, void 0, parseSpecialReference(ctx, 5), void 0, void 0, void 0);
}
function createPlugin(plugin) {
	return plugin;
}
function dedupePlugins(deduped, plugins) {
	for (let i = 0, len = plugins.length; i < len; i++) {
		const current = plugins[i];
		if (!deduped.has(current)) {
			deduped.add(current);
			if (current.extends) dedupePlugins(deduped, current.extends);
		}
	}
}
function resolvePlugins(plugins) {
	if (plugins) {
		const deduped = /* @__PURE__ */ new Set();
		dedupePlugins(deduped, plugins);
		return [...deduped];
	}
}
function isValidKey(key) {
	switch (key) {
		case "constructor":
		case "__proto__":
		case "prototype":
		case "__defineGetter__":
		case "__defineSetter__":
		case "__lookupGetter__":
		case "__lookupSetter__": return false;
		default: return true;
	}
}
var RETURN = () => T;
var SERIALIZED_RETURN = /* @__PURE__ */ RETURN.toString();
var IS_MODERN = /* @__PURE__ */ /=>/.test(SERIALIZED_RETURN);
function createFunction(parameters, body) {
	if (IS_MODERN) return (parameters.length === 1 ? parameters[0] : "(" + parameters.join(",") + ")") + "=>" + (body.startsWith("{") ? "(" + body + ")" : body);
	return "function(" + parameters.join(",") + "){return " + body + "}";
}
function createEffectfulFunction(parameters, body) {
	if (IS_MODERN) return (parameters.length === 1 ? parameters[0] : "(" + parameters.join(",") + ")") + "=>{" + body + "}";
	return "function(" + parameters.join(",") + "){" + body + "}";
}
var REF_START_CHARS = "hjkmoquxzABCDEFGHIJKLNPQRTUVWXYZ$_";
var REF_START_CHARS_LEN = 34;
var REF_CHARS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789$_";
var REF_CHARS_LEN = 64;
function getIdentifier(index) {
	let mod = index % REF_START_CHARS_LEN;
	let ref = REF_START_CHARS[mod];
	index = (index - mod) / REF_START_CHARS_LEN;
	while (index > 0) {
		mod = index % REF_CHARS_LEN;
		ref += REF_CHARS[mod];
		index = (index - mod) / REF_CHARS_LEN;
	}
	return ref;
}
var IDENTIFIER_CHECK = /^[$A-Z_][0-9A-Z_$]*$/i;
function isValidIdentifier(name) {
	const char = name[0];
	return (char === "$" || char === "_" || char >= "A" && char <= "Z" || char >= "a" && char <= "z") && IDENTIFIER_CHECK.test(name);
}
function getAssignmentExpression(assignment) {
	switch (assignment.t) {
		case 0: return assignment.s + "=" + assignment.v;
		case 2: return assignment.s + ".set(" + assignment.k + "," + assignment.v + ")";
		case 1: return assignment.s + ".add(" + assignment.v + ")";
		case 3: return assignment.s + ".delete(" + assignment.k + ")";
		case 4: return "Object.defineProperty(" + assignment.s + ",\"__proto__\",{value:" + assignment.k + ",configurable:!0,enumerable:!0,writable:!0})";
	}
}
function mergeAssignments(assignments) {
	const newAssignments = [];
	let current = assignments[0];
	for (let i = 1, len = assignments.length, item, prev = current; i < len; i++) {
		item = assignments[i];
		if (item.t === 0 && item.v === prev.v) current = {
			t: 0,
			s: item.s,
			k: void 0,
			v: getAssignmentExpression(current)
		};
		else if (item.t === 2 && item.s === prev.s) current = {
			t: 2,
			s: getAssignmentExpression(current),
			k: item.k,
			v: item.v
		};
		else if (item.t === 1 && item.s === prev.s) current = {
			t: 1,
			s: getAssignmentExpression(current),
			k: void 0,
			v: item.v
		};
		else if (item.t === 3 && item.s === prev.s) current = {
			t: 3,
			s: getAssignmentExpression(current),
			k: item.k,
			v: void 0
		};
		else {
			newAssignments.push(current);
			current = item;
		}
		prev = item;
	}
	newAssignments.push(current);
	return newAssignments;
}
function resolveAssignments(assignments) {
	if (assignments.length) {
		let result = "";
		const merged = mergeAssignments(assignments);
		for (let i = 0, len = merged.length; i < len; i++) result += getAssignmentExpression(merged[i]) + ",";
		return result;
	}
}
var NULL_CONSTRUCTOR = "Object.create(null)";
var SET_CONSTRUCTOR = "new Set";
var MAP_CONSTRUCTOR = "new Map";
var PROMISE_RESOLVE = "Promise.resolve";
var PROMISE_REJECT = "Promise.reject";
var OBJECT_FLAG_CONSTRUCTOR = {
	[3]: "Object.freeze",
	[2]: "Object.seal",
	[1]: "Object.preventExtensions",
	[0]: void 0
};
function createBaseSerializerContext(mode, options) {
	return {
		mode,
		plugins: options.plugins,
		features: options.features,
		marked: new Set(options.markedRefs),
		stack: [],
		flags: [],
		assignments: []
	};
}
function createCrossSerializerContext(options) {
	return {
		mode: 2,
		base: createBaseSerializerContext(2, options),
		state: options,
		child: void 0
	};
}
var SerializePluginContext = class {
	constructor(_p) {
		this._p = _p;
	}
	serialize(node) {
		return serialize$1(this._p, node);
	}
};
/**
* Creates the reference param (identifier) from the given reference ID
* Calling this function means the value has been referenced somewhere
*/
function getVanillaRefParam(state, index) {
	/**
	* Creates a new reference ID from a given reference ID
	* This new reference ID means that the reference itself
	* has been referenced at least once, and is used to generate
	* the variables
	*/
	let actualIndex = state.valid.get(index);
	if (actualIndex == null) {
		actualIndex = state.valid.size;
		state.valid.set(index, actualIndex);
	}
	let identifier = state.vars[actualIndex];
	if (identifier == null) {
		identifier = getIdentifier(actualIndex);
		state.vars[actualIndex] = identifier;
	}
	return identifier;
}
function getCrossRefParam(id) {
	return "$R[" + id + "]";
}
/**
* Converts the ID of a reference into a identifier string
* that is used to refer to the object instance in the
* generated script.
*/
function getRefParam(ctx, id) {
	return ctx.mode === 1 ? getVanillaRefParam(ctx.state, id) : getCrossRefParam(id);
}
function markSerializerRef(ctx, id) {
	ctx.marked.add(id);
}
function isSerializerRefMarked(ctx, id) {
	return ctx.marked.has(id);
}
function pushObjectFlag(ctx, flag, id) {
	if (flag !== 0) {
		markSerializerRef(ctx.base, id);
		ctx.base.flags.push({
			type: flag,
			value: getRefParam(ctx, id)
		});
	}
}
function resolveFlags(ctx) {
	let result = "";
	for (let i = 0, current = ctx.flags, len = current.length; i < len; i++) {
		const flag = current[i];
		result += OBJECT_FLAG_CONSTRUCTOR[flag.type] + "(" + flag.value + "),";
	}
	return result;
}
function resolvePatches(ctx) {
	const assignments = resolveAssignments(ctx.assignments);
	const flags = resolveFlags(ctx);
	if (assignments) {
		if (flags) return assignments + flags;
		return assignments;
	}
	return flags;
}
/**
* Generates the inlined assignment for the reference
* This is different from the assignments array as this one
* signifies creation rather than mutation
*/
function createAssignment(ctx, source, value) {
	ctx.assignments.push({
		t: 0,
		s: source,
		k: void 0,
		v: value
	});
}
function createAddAssignment(ctx, ref, value) {
	ctx.base.assignments.push({
		t: 1,
		s: getRefParam(ctx, ref),
		k: void 0,
		v: value
	});
}
function createSetAssignment(ctx, ref, key, value) {
	ctx.base.assignments.push({
		t: 2,
		s: getRefParam(ctx, ref),
		k: key,
		v: value
	});
}
function createDeleteAssignment(ctx, ref, key) {
	ctx.base.assignments.push({
		t: 3,
		s: getRefParam(ctx, ref),
		k: key,
		v: void 0
	});
}
function createArrayAssign(ctx, ref, index, value) {
	createAssignment(ctx.base, getRefParam(ctx, ref) + "[" + index + "]", value);
}
function createObjectAssign(ctx, ref, key, value) {
	if (!isValidKey(key)) {
		ctx.base.assignments.push({
			t: 4,
			s: getRefParam(ctx, ref),
			k: value,
			v: void 0
		});
		return;
	}
	createAssignment(ctx.base, getRefParam(ctx, ref) + "." + key, value);
}
function createSequenceAssign(ctx, ref, index, value) {
	createAssignment(ctx.base, getRefParam(ctx, ref) + ".v[" + index + "]", value);
}
/**
* Checks if the value is in the stack. Stack here is a reference
* structure to know if a object is to be accessed in a TDZ.
*/
function isIndexedValueInStack(ctx, node) {
	return node.t === 4 && ctx.stack.includes(node.i);
}
/**
* Produces an assignment expression. `id` generates a reference
* parameter (through `getRefParam`) and has the option to
* return the reference parameter directly or assign a value to
* it.
*/
function assignIndexedValue(ctx, index, value) {
	if (ctx.mode === 1 && !isSerializerRefMarked(ctx.base, index)) return value;
	/**
	* In cross-reference, we have to assume that
	* every reference are going to be referenced
	* in the future, and so we need to store
	* all of it into the reference array.
	*
	* otherwise in vanilla, we only do this if it
	* is actually referenced
	*/
	return getRefParam(ctx, index) + "=" + value;
}
function serializeReference(node) {
	return "__SEROVAL_REFS__.get(\"" + node.s + "\")";
}
function serializeArrayItem(ctx, id, item, index) {
	if (item) {
		if (isIndexedValueInStack(ctx.base, item)) {
			markSerializerRef(ctx.base, id);
			createArrayAssign(ctx, id, index, getRefParam(ctx, item.i));
			return "";
		}
		return serialize$1(ctx, item);
	}
	return "";
}
function serializeArray(ctx, node) {
	const id = node.i;
	const list = node.a;
	const len = list.length;
	if (len > 0) {
		ctx.base.stack.push(id);
		let values = serializeArrayItem(ctx, id, list[0], 0);
		let isHoley = values === "";
		for (let i = 1, item; i < len; i++) {
			item = serializeArrayItem(ctx, id, list[i], i);
			values += "," + item;
			isHoley = item === "";
		}
		ctx.base.stack.pop();
		pushObjectFlag(ctx, node.o, node.i);
		return "[" + values + (isHoley ? ",]" : "]");
	}
	return "[]";
}
function serializeProperty(ctx, source, key, val) {
	if (typeof key === "string") {
		const check = Number(key);
		const isIdentifier = check >= 0 && check.toString() === key || isValidIdentifier(key);
		if (isIndexedValueInStack(ctx.base, val)) {
			const refParam = getRefParam(ctx, val.i);
			markSerializerRef(ctx.base, source.i);
			if (isIdentifier && check !== check) createObjectAssign(ctx, source.i, key, refParam);
			else createArrayAssign(ctx, source.i, isIdentifier ? key : "\"" + key + "\"", refParam);
			return "";
		}
		if (isValidKey(key)) return (isIdentifier ? key : "\"" + key + "\"") + ":" + serialize$1(ctx, val);
		return "[\"" + key + "\"]:" + serialize$1(ctx, val);
	}
	return "[" + serialize$1(ctx, key) + "]:" + serialize$1(ctx, val);
}
function serializeProperties(ctx, source, record) {
	const keys = record.k;
	const len = keys.length;
	if (len > 0) {
		const values = record.v;
		ctx.base.stack.push(source.i);
		let result = serializeProperty(ctx, source, keys[0], values[0]);
		for (let i = 1, item = result; i < len; i++) {
			item = serializeProperty(ctx, source, keys[i], values[i]);
			result += (item && result && ",") + item;
		}
		ctx.base.stack.pop();
		return "{" + result + "}";
	}
	return "{}";
}
function serializeObject(ctx, node) {
	pushObjectFlag(ctx, node.o, node.i);
	return serializeProperties(ctx, node, node.p);
}
function serializeWithObjectAssign(ctx, source, value, serialized) {
	const fields = serializeProperties(ctx, source, value);
	if (fields !== "{}") return "Object.assign(" + serialized + "," + fields + ")";
	return serialized;
}
function serializeStringKeyAssignment(ctx, source, mainAssignments, key, value) {
	const base = ctx.base;
	const serialized = serialize$1(ctx, value);
	const check = Number(key);
	const isIdentifier = check >= 0 && check.toString() === key || isValidIdentifier(key);
	if (isIndexedValueInStack(base, value)) {
		if (isIdentifier && check !== check) createObjectAssign(ctx, source.i, key, serialized);
		else createArrayAssign(ctx, source.i, isIdentifier ? key : "\"" + key + "\"", serialized);
	} else {
		const parentAssignment = base.assignments;
		base.assignments = mainAssignments;
		if (isIdentifier && check !== check) createObjectAssign(ctx, source.i, key, serialized);
		else createArrayAssign(ctx, source.i, isIdentifier ? key : "\"" + key + "\"", serialized);
		base.assignments = parentAssignment;
	}
}
function serializeAssignment(ctx, source, mainAssignments, key, value) {
	if (typeof key === "string") serializeStringKeyAssignment(ctx, source, mainAssignments, key, value);
	else {
		const base = ctx.base;
		const parent = base.stack;
		base.stack = [];
		const serialized = serialize$1(ctx, value);
		base.stack = parent;
		const parentAssignment = base.assignments;
		base.assignments = mainAssignments;
		createArrayAssign(ctx, source.i, serialize$1(ctx, key), serialized);
		base.assignments = parentAssignment;
	}
}
function serializeAssignments(ctx, source, node) {
	const keys = node.k;
	const len = keys.length;
	if (len > 0) {
		const mainAssignments = [];
		const values = node.v;
		ctx.base.stack.push(source.i);
		for (let i = 0; i < len; i++) serializeAssignment(ctx, source, mainAssignments, keys[i], values[i]);
		ctx.base.stack.pop();
		return resolveAssignments(mainAssignments);
	}
}
function serializeDictionary(ctx, node, init) {
	if (node.p) {
		const base = ctx.base;
		if (base.features & 8) init = serializeWithObjectAssign(ctx, node, node.p, init);
		else {
			markSerializerRef(base, node.i);
			const assignments = serializeAssignments(ctx, node, node.p);
			if (assignments) return "(" + assignIndexedValue(ctx, node.i, init) + "," + assignments + getRefParam(ctx, node.i) + ")";
		}
	}
	return init;
}
function serializeNullConstructor(ctx, node) {
	pushObjectFlag(ctx, node.o, node.i);
	return serializeDictionary(ctx, node, NULL_CONSTRUCTOR);
}
function serializeDate(node) {
	return "new Date(\"" + node.s + "\")";
}
var TEMPORAL_CONSTRUCTOR = {
	[0]: "Temporal.Instant",
	[1]: "Temporal.Duration",
	[2]: "Temporal.PlainDate",
	[3]: "Temporal.PlainDateTime",
	[4]: "Temporal.PlainMonthDay",
	[5]: "Temporal.PlainTime",
	[6]: "Temporal.PlainYearMonth",
	[7]: "Temporal.ZonedDateTime"
};
function serializeTemporal(ctx, node) {
	if (ctx.base.features & 64) return TEMPORAL_CONSTRUCTOR[node.c] + ".from(\"" + node.s + "\")";
	throw new SerovalUnsupportedNodeError(node);
}
function serializeRegExp(ctx, node) {
	if (ctx.base.features & 32) return "/" + deserializeString(node.c) + "/" + node.m;
	throw new SerovalUnsupportedNodeError(node);
}
function serializeSetItem(ctx, id, item) {
	const base = ctx.base;
	if (isIndexedValueInStack(base, item)) {
		markSerializerRef(base, id);
		createAddAssignment(ctx, id, getRefParam(ctx, item.i));
		return "";
	}
	return serialize$1(ctx, item);
}
function serializeSet(ctx, node) {
	let serialized = SET_CONSTRUCTOR;
	const items = node.a;
	const size = items.length;
	const id = node.i;
	if (size > 0) {
		ctx.base.stack.push(id);
		let result = serializeSetItem(ctx, id, items[0]);
		for (let i = 1, item = result; i < size; i++) {
			item = serializeSetItem(ctx, id, items[i]);
			result += (item && result && ",") + item;
		}
		ctx.base.stack.pop();
		if (result) serialized += "([" + result + "])";
	}
	return serialized;
}
function serializeMapEntry(ctx, id, key, val, sentinel) {
	const base = ctx.base;
	if (isIndexedValueInStack(base, key)) {
		const keyRef = getRefParam(ctx, key.i);
		markSerializerRef(base, id);
		if (isIndexedValueInStack(base, val)) {
			createSetAssignment(ctx, id, keyRef, getRefParam(ctx, val.i));
			return "";
		}
		if (val.t !== 4 && val.i != null && isSerializerRefMarked(base, val.i)) {
			const serialized = "(" + serialize$1(ctx, val) + ",[" + sentinel + "," + sentinel + "])";
			createSetAssignment(ctx, id, keyRef, getRefParam(ctx, val.i));
			createDeleteAssignment(ctx, id, sentinel);
			return serialized;
		}
		const parent = base.stack;
		base.stack = [];
		createSetAssignment(ctx, id, keyRef, serialize$1(ctx, val));
		base.stack = parent;
		return "";
	}
	if (isIndexedValueInStack(base, val)) {
		const valueRef = getRefParam(ctx, val.i);
		markSerializerRef(base, id);
		if (key.t !== 4 && key.i != null && isSerializerRefMarked(base, key.i)) {
			const serialized = "(" + serialize$1(ctx, key) + ",[" + sentinel + "," + sentinel + "])";
			createSetAssignment(ctx, id, getRefParam(ctx, key.i), valueRef);
			createDeleteAssignment(ctx, id, sentinel);
			return serialized;
		}
		const parent = base.stack;
		base.stack = [];
		createSetAssignment(ctx, id, serialize$1(ctx, key), valueRef);
		base.stack = parent;
		return "";
	}
	return "[" + serialize$1(ctx, key) + "," + serialize$1(ctx, val) + "]";
}
function serializeMap(ctx, node) {
	let serialized = MAP_CONSTRUCTOR;
	const keys = node.e.k;
	const size = keys.length;
	const id = node.i;
	const sentinel = node.f;
	const sentinelId = getRefParam(ctx, sentinel.i);
	const base = ctx.base;
	if (size > 0) {
		const vals = node.e.v;
		base.stack.push(id);
		let result = serializeMapEntry(ctx, id, keys[0], vals[0], sentinelId);
		for (let i = 1, item = result; i < size; i++) {
			item = serializeMapEntry(ctx, id, keys[i], vals[i], sentinelId);
			result += (item && result && ",") + item;
		}
		base.stack.pop();
		if (result) serialized += "([" + result + "])";
	}
	if (sentinel.t === 26) {
		markSerializerRef(base, sentinel.i);
		serialized = "(" + serialize$1(ctx, sentinel) + "," + serialized + ")";
	}
	return serialized;
}
function serializeArrayBuffer(ctx, node) {
	return getConstructor(ctx, node.f) + "(\"" + node.s + "\")";
}
function serializeTypedArray(ctx, node) {
	return "new " + node.c + "(" + serialize$1(ctx, node.f) + "," + node.b + "," + node.l + ")";
}
function serializeDataView(ctx, node) {
	return "new DataView(" + serialize$1(ctx, node.f) + "," + node.b + "," + node.l + ")";
}
function serializeAggregateError(ctx, node) {
	const id = node.i;
	ctx.base.stack.push(id);
	const serialized = serializeDictionary(ctx, node, "new AggregateError([],\"" + node.m + "\")");
	ctx.base.stack.pop();
	return serialized;
}
function serializeError(ctx, node) {
	return serializeDictionary(ctx, node, "new " + ERROR_CONSTRUCTOR_STRING[node.s] + "(\"" + node.m + "\")");
}
function serializePromise(ctx, node) {
	let serialized;
	const fulfilled = node.f;
	const id = node.i;
	const promiseConstructor = node.s ? PROMISE_RESOLVE : PROMISE_REJECT;
	const base = ctx.base;
	if (isIndexedValueInStack(base, fulfilled)) {
		const ref = getRefParam(ctx, fulfilled.i);
		serialized = promiseConstructor + (node.s ? "().then(" + createFunction([], ref) + ")" : "().catch(" + createEffectfulFunction([], "throw " + ref) + ")");
	} else {
		base.stack.push(id);
		const result = serialize$1(ctx, fulfilled);
		base.stack.pop();
		serialized = promiseConstructor + "(" + result + ")";
	}
	return serialized;
}
function serializeBoxed(ctx, node) {
	return "Object(" + serialize$1(ctx, node.f) + ")";
}
function getConstructor(ctx, node) {
	const current = serialize$1(ctx, node);
	return node.t === 4 ? current : "(" + current + ")";
}
function serializePromiseConstructor(ctx, node) {
	if (ctx.mode === 1) throw new SerovalUnsupportedNodeError(node);
	return "(" + assignIndexedValue(ctx, node.s, getConstructor(ctx, node.f) + "()") + ").p";
}
function serializePromiseResolve(ctx, node) {
	if (ctx.mode === 1) throw new SerovalUnsupportedNodeError(node);
	return getConstructor(ctx, node.a[0]) + "(" + getRefParam(ctx, node.i) + "," + serialize$1(ctx, node.a[1]) + ")";
}
function serializePromiseReject(ctx, node) {
	if (ctx.mode === 1) throw new SerovalUnsupportedNodeError(node);
	return getConstructor(ctx, node.a[0]) + "(" + getRefParam(ctx, node.i) + "," + serialize$1(ctx, node.a[1]) + ")";
}
function serializePlugin(ctx, node) {
	const currentPlugins = ctx.base.plugins;
	if (currentPlugins) for (let i = 0, len = currentPlugins.length; i < len; i++) {
		const plugin = currentPlugins[i];
		if (plugin.tag === node.c) {
			if (ctx.child == null) ctx.child = new SerializePluginContext(ctx);
			return plugin.serialize(node.s, ctx.child, { id: node.i });
		}
	}
	throw new SerovalMissingPluginError(node.c);
}
function serializeIteratorFactory(ctx, node) {
	let result = "";
	let initialized = false;
	if (node.f.t !== 4) {
		markSerializerRef(ctx.base, node.f.i);
		result = "(" + serialize$1(ctx, node.f) + ",";
		initialized = true;
	}
	result += assignIndexedValue(ctx, node.i, "(" + SERIALIZED_ITERATOR_CONSTRUCTOR + ")(" + getRefParam(ctx, node.f.i) + ")");
	if (initialized) result += ")";
	return result;
}
function serializeIteratorFactoryInstance(ctx, node) {
	return getConstructor(ctx, node.a[0]) + "(" + serialize$1(ctx, node.a[1]) + ")";
}
function serializeAsyncIteratorFactory(ctx, node) {
	const promise = node.a[0];
	const symbol = node.a[1];
	const base = ctx.base;
	let result = "";
	if (promise.t !== 4) {
		markSerializerRef(base, promise.i);
		result += "(" + serialize$1(ctx, promise);
	}
	if (symbol.t !== 4) {
		markSerializerRef(base, symbol.i);
		result += (result ? "," : "(") + serialize$1(ctx, symbol);
	}
	if (result) result += ",";
	const iterator = assignIndexedValue(ctx, node.i, "(" + SERIALIZED_ASYNC_ITERATOR_CONSTRUCTOR + ")(" + getRefParam(ctx, symbol.i) + "," + getRefParam(ctx, promise.i) + ")");
	if (result) return result + iterator + ")";
	return iterator;
}
function serializeAsyncIteratorFactoryInstance(ctx, node) {
	return getConstructor(ctx, node.a[0]) + "(" + serialize$1(ctx, node.a[1]) + ")";
}
function serializeStreamConstructor(ctx, node) {
	const result = assignIndexedValue(ctx, node.i, getConstructor(ctx, node.f) + "()");
	const len = node.a.length;
	if (len) {
		let values = serialize$1(ctx, node.a[0]);
		for (let i = 1; i < len; i++) values += "," + serialize$1(ctx, node.a[i]);
		return "(" + result + "," + values + "," + getRefParam(ctx, node.i) + ")";
	}
	return result;
}
function serializeStreamNext(ctx, node) {
	return getRefParam(ctx, node.i) + ".next(" + serialize$1(ctx, node.f) + ")";
}
function serializeStreamThrow(ctx, node) {
	return getRefParam(ctx, node.i) + ".throw(" + serialize$1(ctx, node.f) + ")";
}
function serializeStreamReturn(ctx, node) {
	return getRefParam(ctx, node.i) + ".return(" + serialize$1(ctx, node.f) + ")";
}
function serializeSequenceItem(ctx, id, index, item) {
	const base = ctx.base;
	if (isIndexedValueInStack(base, item)) {
		markSerializerRef(base, id);
		createSequenceAssign(ctx, id, index, getRefParam(ctx, item.i));
		return "";
	}
	return serialize$1(ctx, item);
}
function serializeSequence(ctx, node) {
	const items = node.a;
	const size = items.length;
	const id = node.i;
	if (size > 0) {
		ctx.base.stack.push(id);
		let result = serializeSequenceItem(ctx, id, 0, items[0]);
		for (let i = 1, item = result; i < size; i++) {
			item = serializeSequenceItem(ctx, id, i, items[i]);
			result += (item && result && ",") + item;
		}
		ctx.base.stack.pop();
		if (result) return "{__SEROVAL_SEQUENCE__:!0,v:[" + result + "],t:" + node.s + ",d:" + node.l + "}";
	}
	return "{__SEROVAL_SEQUENCE__:!0,v:[],t:-1,d:0}";
}
function serializeAssignable(ctx, node) {
	switch (node.t) {
		case 17: return SYMBOL_STRING[node.s];
		case 18: return serializeReference(node);
		case 9: return serializeArray(ctx, node);
		case 10: return serializeObject(ctx, node);
		case 11: return serializeNullConstructor(ctx, node);
		case 5: return serializeDate(node);
		case 6: return serializeRegExp(ctx, node);
		case 7: return serializeSet(ctx, node);
		case 8: return serializeMap(ctx, node);
		case 19: return serializeArrayBuffer(ctx, node);
		case 16:
		case 15: return serializeTypedArray(ctx, node);
		case 20: return serializeDataView(ctx, node);
		case 14: return serializeAggregateError(ctx, node);
		case 13: return serializeError(ctx, node);
		case 12: return serializePromise(ctx, node);
		case 21: return serializeBoxed(ctx, node);
		case 22: return serializePromiseConstructor(ctx, node);
		case 25: return serializePlugin(ctx, node);
		case 26: return SPECIAL_REF_STRING[node.s];
		case 35: return serializeSequence(ctx, node);
		case 36: return serializeTemporal(ctx, node);
		default: throw new SerovalUnsupportedNodeError(node);
	}
}
function serialize$1(ctx, node) {
	switch (node.t) {
		case 2: return CONSTANT_STRING[node.s];
		case 0: return "" + node.s;
		case 1: return "\"" + node.s + "\"";
		case 3: return node.s + "n";
		case 4: return getRefParam(ctx, node.i);
		case 23: return serializePromiseResolve(ctx, node);
		case 24: return serializePromiseReject(ctx, node);
		case 27: return serializeIteratorFactory(ctx, node);
		case 28: return serializeIteratorFactoryInstance(ctx, node);
		case 29: return serializeAsyncIteratorFactory(ctx, node);
		case 30: return serializeAsyncIteratorFactoryInstance(ctx, node);
		case 31: return serializeStreamConstructor(ctx, node);
		case 32: return serializeStreamNext(ctx, node);
		case 33: return serializeStreamThrow(ctx, node);
		case 34: return serializeStreamReturn(ctx, node);
		default: return assignIndexedValue(ctx, node.i, serializeAssignable(ctx, node));
	}
}
function serializeTopCross(ctx, tree) {
	const result = serialize$1(ctx, tree);
	const id = tree.i;
	if (id == null) return result;
	const patches = resolvePatches(ctx.base);
	const ref = getRefParam(ctx, id);
	const scopeId = ctx.state.scopeId;
	const params = scopeId == null ? "" : "$R";
	const body = patches ? "(" + result + "," + patches + ref + ")" : result;
	if (params === "") {
		if (tree.t === 10 && !patches) return "(" + body + ")";
		return body;
	}
	const args = scopeId == null ? "()" : "($R[\"" + serializeString(scopeId) + "\"])";
	return "(" + createFunction([params], body) + ")" + args;
}
var SyncParsePluginContext = class {
	constructor(_p, depth) {
		this._p = _p;
		this.depth = depth;
	}
	parse(current) {
		return parseSOS(this._p, this.depth, current);
	}
};
var StreamParsePluginContext = class {
	constructor(_p, depth) {
		this._p = _p;
		this.depth = depth;
	}
	parse(current) {
		return parseSOS(this._p, this.depth, current);
	}
	parseWithError(current) {
		return parseWithError(this._p, this.depth, current);
	}
	isAlive() {
		return this._p.state.alive;
	}
	pushPendingState() {
		pushPendingState(this._p);
	}
	popPendingState() {
		popPendingState(this._p);
	}
	onParse(node) {
		onParse(this._p, node);
	}
	onError(error) {
		onError(this._p, error);
	}
	addCleanup(callback) {
		this._p.state.cleanups.push(callback);
	}
};
function createStreamParserState(options) {
	return {
		alive: true,
		pending: 0,
		initial: true,
		buffer: [],
		onParse: options.onParse,
		onError: options.onError,
		onDone: options.onDone,
		cleanups: []
	};
}
function createStreamParserContext(options) {
	return {
		type: 2,
		base: createBaseParserContext(2, options),
		state: createStreamParserState(options)
	};
}
function parseItems(ctx, depth, current) {
	const nodes = [];
	for (let i = 0, len = current.length; i < len; i++) if (i in current) nodes[i] = parseSOS(ctx, depth, current[i]);
	else nodes[i] = 0;
	return nodes;
}
function parseArray(ctx, depth, id, current) {
	return createArrayNode(id, current, parseItems(ctx, depth, current));
}
function parseProperties(ctx, depth, properties) {
	const entries = Object.entries(properties);
	const keyNodes = [];
	const valueNodes = [];
	for (let i = 0, len = entries.length; i < len; i++) {
		keyNodes.push(serializeString(entries[i][0]));
		valueNodes.push(parseSOS(ctx, depth, entries[i][1]));
	}
	if (SYM_ITERATOR in properties) {
		keyNodes.push(parseWellKnownSymbol(ctx.base, SYM_ITERATOR));
		valueNodes.push(createIteratorFactoryInstanceNode(parseIteratorFactory(ctx.base), parseSOS(ctx, depth, createSequenceFromIterable(properties))));
	}
	if (SYM_ASYNC_ITERATOR in properties) {
		keyNodes.push(parseWellKnownSymbol(ctx.base, SYM_ASYNC_ITERATOR));
		valueNodes.push(createAsyncIteratorFactoryInstanceNode(parseAsyncIteratorFactory(ctx.base), parseSOS(ctx, depth, ctx.type === 1 ? createStream() : createStreamFromAsyncIterable(properties, ctx.state.cleanups))));
	}
	if (SYM_TO_STRING_TAG in properties) {
		keyNodes.push(parseWellKnownSymbol(ctx.base, SYM_TO_STRING_TAG));
		valueNodes.push(createStringNode(properties[SYM_TO_STRING_TAG]));
	}
	if (SYM_IS_CONCAT_SPREADABLE in properties) {
		keyNodes.push(parseWellKnownSymbol(ctx.base, SYM_IS_CONCAT_SPREADABLE));
		valueNodes.push(properties[SYM_IS_CONCAT_SPREADABLE] ? TRUE_NODE : FALSE_NODE);
	}
	return {
		k: keyNodes,
		v: valueNodes
	};
}
function parsePlainObject(ctx, depth, id, current, empty) {
	return createObjectNode(id, current, empty, parseProperties(ctx, depth, current));
}
function parseBoxed(ctx, depth, id, current) {
	return createBoxedNode(id, parseSOS(ctx, depth, current.valueOf()));
}
function parseTypedArray(ctx, depth, id, current) {
	current = getArrayBufferView(ctx.base, current);
	return createTypedArrayNode(id, current, parseSOS(ctx, depth, current.buffer));
}
function parseBigIntTypedArray(ctx, depth, id, current) {
	current = getArrayBufferView(ctx.base, current);
	return createBigIntTypedArrayNode(id, current, parseSOS(ctx, depth, current.buffer));
}
function parseDataView(ctx, depth, id, current) {
	current = getArrayBufferView(ctx.base, current);
	return createDataViewNode(id, current, parseSOS(ctx, depth, current.buffer));
}
function parseError(ctx, depth, id, current) {
	const options = getErrorOptions(current, ctx.base.features);
	return createErrorNode(id, current, options ? parseProperties(ctx, depth, options) : void 0);
}
function parseAggregateError(ctx, depth, id, current) {
	const options = getErrorOptions(current, ctx.base.features);
	return createAggregateErrorNode(id, current, options ? parseProperties(ctx, depth, options) : void 0);
}
function parseMap(ctx, depth, id, current) {
	const keyNodes = [];
	const valueNodes = [];
	for (const [key, value] of current.entries()) {
		keyNodes.push(parseSOS(ctx, depth, key));
		valueNodes.push(parseSOS(ctx, depth, value));
	}
	return createMapNode(ctx.base, id, keyNodes, valueNodes);
}
function parseSet(ctx, depth, id, current) {
	const items = [];
	for (const item of current.keys()) items.push(parseSOS(ctx, depth, item));
	return createSetNode(id, items);
}
function parseStream(ctx, depth, id, current) {
	const result = createStreamConstructorNode(id, parseSpecialReference(ctx.base, 4), []);
	if (ctx.type === 1) return result;
	pushPendingState(ctx);
	current.on({
		next: (value) => {
			if (ctx.state.alive) {
				const parsed = parseWithError(ctx, depth, value);
				if (parsed) onParse(ctx, createStreamNextNode(id, parsed));
			}
		},
		throw: (value) => {
			if (ctx.state.alive) {
				const parsed = parseWithError(ctx, depth, value);
				if (parsed) onParse(ctx, createStreamThrowNode(id, parsed));
			}
			popPendingState(ctx);
		},
		return: (value) => {
			if (ctx.state.alive) {
				const parsed = parseWithError(ctx, depth, value);
				if (parsed) onParse(ctx, createStreamReturnNode(id, parsed));
			}
			popPendingState(ctx);
		}
	});
	return result;
}
function handlePromiseSuccess(id, depth, data) {
	if (this.state.alive) {
		const parsed = parseWithError(this, depth, data);
		if (parsed) onParse(this, createSerovalNode(23, id, void 0, void 0, void 0, void 0, void 0, [parseSpecialReference(this.base, 2), parsed], void 0, void 0, void 0, void 0));
		popPendingState(this);
	}
}
function handlePromiseFailure(id, depth, data) {
	if (this.state.alive) {
		const parsed = parseWithError(this, depth, data);
		if (parsed) onParse(this, createSerovalNode(24, id, void 0, void 0, void 0, void 0, void 0, [parseSpecialReference(this.base, 3), parsed], void 0, void 0, void 0, void 0));
	}
	popPendingState(this);
}
function parsePromise(ctx, depth, id, current) {
	const resolver = createIndexForValue(ctx.base, {});
	if (ctx.type === 2) {
		pushPendingState(ctx);
		current.then(handlePromiseSuccess.bind(ctx, resolver, depth), handlePromiseFailure.bind(ctx, resolver, depth));
	}
	return createPromiseConstructorNode(ctx.base, id, resolver);
}
function parsePluginSync(ctx, depth, id, current, currentPlugins) {
	for (let i = 0, len = currentPlugins.length; i < len; i++) {
		const plugin = currentPlugins[i];
		if (plugin.parse.sync && plugin.test(current)) return createPluginNode(id, plugin.tag, plugin.parse.sync(current, new SyncParsePluginContext(ctx, depth), { id }));
	}
}
function parsePluginStream(ctx, depth, id, current, currentPlugins) {
	for (let i = 0, len = currentPlugins.length; i < len; i++) {
		const plugin = currentPlugins[i];
		if (plugin.parse.stream && plugin.test(current)) return createPluginNode(id, plugin.tag, plugin.parse.stream(current, new StreamParsePluginContext(ctx, depth), { id }));
	}
}
function parsePlugin(ctx, depth, id, current) {
	const currentPlugins = ctx.base.plugins;
	if (currentPlugins) return ctx.type === 1 ? parsePluginSync(ctx, depth, id, current, currentPlugins) : parsePluginStream(ctx, depth, id, current, currentPlugins);
}
function parseSequence(ctx, depth, id, current) {
	const nodes = [];
	for (let i = 0, len = current.v.length; i < len; i++) nodes[i] = parseSOS(ctx, depth, current.v[i]);
	return createSequenceNode(id, nodes, current.t, current.d);
}
function parseObjectPhase2(ctx, depth, id, current, currentClass) {
	switch (currentClass) {
		case Object: return parsePlainObject(ctx, depth, id, current, false);
		case void 0: return parsePlainObject(ctx, depth, id, current, true);
		case Date: return createDateNode(id, current);
		case Error:
		case EvalError:
		case RangeError:
		case ReferenceError:
		case SyntaxError:
		case TypeError:
		case URIError: return parseError(ctx, depth, id, current);
		case Number:
		case Boolean:
		case String:
		case BigInt: return parseBoxed(ctx, depth, id, current);
		case ArrayBuffer: return createArrayBufferNode(ctx.base, id, current);
		case Int8Array:
		case Int16Array:
		case Int32Array:
		case Uint8Array:
		case Uint16Array:
		case Uint32Array:
		case Uint8ClampedArray:
		case Float32Array:
		case Float64Array: return parseTypedArray(ctx, depth, id, current);
		case DataView: return parseDataView(ctx, depth, id, current);
		case Map: return parseMap(ctx, depth, id, current);
		case Set: return parseSet(ctx, depth, id, current);
	}
	if (currentClass === Promise || current instanceof Promise) return parsePromise(ctx, depth, id, current);
	const currentFeatures = ctx.base.features;
	if (currentFeatures & 32 && currentClass === RegExp) return createRegExpNode(id, current);
	if (currentFeatures & 16) switch (currentClass) {
		case BigInt64Array:
		case BigUint64Array: return parseBigIntTypedArray(ctx, depth, id, current);
	}
	if (currentFeatures & 1 && typeof AggregateError !== "undefined" && (currentClass === AggregateError || current instanceof AggregateError)) return parseAggregateError(ctx, depth, id, current);
	if (currentFeatures & 64 && typeof Temporal !== "undefined") switch (currentClass) {
		case Temporal.Instant: return createTemporalNode(id, 0, current);
		case Temporal.Duration: return createTemporalNode(id, 1, current);
		case Temporal.PlainDate: return createTemporalNode(id, 2, current);
		case Temporal.PlainDateTime: return createTemporalNode(id, 3, current);
		case Temporal.PlainMonthDay: return createTemporalNode(id, 4, current);
		case Temporal.PlainTime: return createTemporalNode(id, 5, current);
		case Temporal.PlainYearMonth: return createTemporalNode(id, 6, current);
		case Temporal.ZonedDateTime: return createTemporalNode(id, 7, current);
	}
	if (current instanceof Error) return parseError(ctx, depth, id, current);
	if (SYM_ITERATOR in current || SYM_ASYNC_ITERATOR in current) return parsePlainObject(ctx, depth, id, current, !!currentClass);
	throw new SerovalUnsupportedTypeError(current);
}
function parseObject(ctx, depth, id, current) {
	if (Array.isArray(current)) return parseArray(ctx, depth, id, current);
	if (isStream(current)) return parseStream(ctx, depth, id, current);
	if (isSequence(current)) return parseSequence(ctx, depth, id, current);
	let currentClass = current.constructor;
	if (currentClass !== void 0 && typeof currentClass !== "function") {
		const proto = Object.getPrototypeOf(current);
		currentClass = proto === null ? void 0 : proto.constructor;
	}
	if (currentClass === OpaqueReference) return parseSOS(ctx, depth, current.replacement);
	const parsed = parsePlugin(ctx, depth, id, current);
	if (parsed) return parsed;
	return parseObjectPhase2(ctx, depth, id, current, currentClass);
}
function parseFunction(ctx, depth, current) {
	const ref = getReferenceNode(ctx.base, current);
	if (ref.type !== 0) return ref.value;
	const plugin = parsePlugin(ctx, depth, ref.value, current);
	if (plugin) return plugin;
	throw new SerovalUnsupportedTypeError(current);
}
function parseSOS(ctx, depth, current) {
	if (depth >= ctx.base.depthLimit) throw new SerovalDepthLimitError(ctx.base.depthLimit);
	switch (typeof current) {
		case "boolean": return current ? TRUE_NODE : FALSE_NODE;
		case "undefined": return UNDEFINED_NODE;
		case "string": return createStringNode(current);
		case "number": return createNumberNode(current);
		case "bigint": return createBigIntNode(current);
		case "object":
			if (current) {
				const ref = getReferenceNode(ctx.base, current);
				return ref.type === 0 ? parseObject(ctx, depth + 1, ref.value, current) : ref.value;
			}
			return NULL_NODE;
		case "symbol": return parseWellKnownSymbol(ctx.base, current);
		case "function": return parseFunction(ctx, depth, current);
		default: throw new SerovalUnsupportedTypeError(current);
	}
}
function onParse(ctx, node) {
	if (ctx.state.initial) ctx.state.buffer.push(node);
	else onParseInternal(ctx, node, false);
}
function onError(ctx, error) {
	if (ctx.state.onError) ctx.state.onError(error);
	else throw error instanceof SerovalParserError ? error : new SerovalParserError(error);
}
function onDone(ctx) {
	if (ctx.state.onDone) ctx.state.onDone();
	for (let i = 0, len = ctx.state.cleanups.length; i < len; i++) ctx.state.cleanups[i]();
}
function onParseInternal(ctx, node, initial) {
	try {
		ctx.state.onParse(node, initial);
	} catch (error) {
		onError(ctx, error);
	}
}
function pushPendingState(ctx) {
	ctx.state.pending++;
}
function popPendingState(ctx) {
	if (--ctx.state.pending <= 0) onDone(ctx);
}
function parseWithError(ctx, depth, current) {
	try {
		return parseSOS(ctx, depth, current);
	} catch (err) {
		onError(ctx, err);
		return;
	}
}
function startStreamParse(ctx, current) {
	const parsed = parseWithError(ctx, 0, current);
	if (parsed) {
		onParseInternal(ctx, parsed, true);
		ctx.state.initial = false;
		flushStreamParse(ctx, ctx.state);
		if (ctx.state.pending <= 0) destroyStreamParse(ctx);
	}
}
function flushStreamParse(ctx, state) {
	for (let i = 0, len = state.buffer.length; i < len; i++) onParseInternal(ctx, state.buffer[i], false);
}
function destroyStreamParse(ctx) {
	if (ctx.state.alive) {
		onDone(ctx);
		ctx.state.alive = false;
	}
}
function crossSerializeStream(source, options) {
	const plugins = resolvePlugins(options.plugins);
	const ctx = createStreamParserContext({
		compactArrayBufferViews: options.compactArrayBufferViews,
		plugins,
		refs: options.refs,
		disabledFeatures: options.disabledFeatures,
		onParse(node, initial) {
			const serial = createCrossSerializerContext({
				plugins,
				features: ctx.base.features,
				scopeId: options.scopeId,
				markedRefs: ctx.base.marked
			});
			let serialized;
			try {
				serialized = serializeTopCross(serial, node);
			} catch (err) {
				if (options.onError) options.onError(err);
				return;
			}
			options.onSerialize(serialized, initial);
		},
		onError: options.onError,
		onDone: options.onDone
	});
	startStreamParse(ctx, source);
	return destroyStreamParse.bind(null, ctx);
}
var Serializer = class {
	constructor(options) {
		this.options = options;
		this.alive = true;
		this.flushed = false;
		this.done = false;
		this.pending = 0;
		this.cleanups = [];
		this.refs = /* @__PURE__ */ new Map();
		this.keys = /* @__PURE__ */ new Set();
		this.ids = 0;
		this.plugins = resolvePlugins(options.plugins);
	}
	write(key, value) {
		if (this.alive && !this.flushed) {
			this.pending++;
			this.keys.add(key);
			this.cleanups.push(crossSerializeStream(value, {
				plugins: this.plugins,
				scopeId: this.options.scopeId,
				refs: this.refs,
				disabledFeatures: this.options.disabledFeatures,
				compactArrayBufferViews: this.options.compactArrayBufferViews,
				onError: this.options.onError,
				onSerialize: (data, initial) => {
					if (this.alive) this.options.onData(initial ? this.options.globalIdentifier + "[\"" + serializeString(key) + "\"]=" + data : data);
				},
				onDone: () => {
					if (this.alive) {
						this.pending--;
						if (this.pending <= 0 && this.flushed && !this.done && this.options.onDone) {
							this.options.onDone();
							this.done = true;
						}
					}
				}
			}));
		}
	}
	getNextID() {
		while (this.keys.has("" + this.ids)) this.ids++;
		return "" + this.ids;
	}
	push(value) {
		const newID = this.getNextID();
		this.write(newID, value);
		return newID;
	}
	flush() {
		if (this.alive) {
			this.flushed = true;
			if (this.pending <= 0 && !this.done && this.options.onDone) {
				this.options.onDone();
				this.done = true;
			}
		}
	}
	close() {
		if (this.alive) {
			for (let i = 0, len = this.cleanups.length; i < len; i++) this.cleanups[i]();
			if (!this.done && this.options.onDone) {
				this.options.onDone();
				this.done = true;
			}
			this.alive = false;
		}
	}
};
var PROMISE_TO_ABORT_SIGNAL = (promise) => {
	const controller = new AbortController();
	const abort = controller.abort.bind(controller);
	promise.then(abort, abort);
	return controller;
};
function resolveAbortSignalResult(resolve) {
	resolve(this.reason);
}
function resolveAbortSignal(resolve) {
	this.addEventListener("abort", resolveAbortSignalResult.bind(this, resolve), { once: true });
}
function abortSignalToPromise(signal) {
	return new Promise(resolveAbortSignal.bind(signal));
}
var ABORT_CONTROLLER = {};
var AbortSignalPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/AbortSignal",
	extends: [/* @__PURE__ */ createPlugin({
		tag: "seroval-plugins/web/AbortControllerFactoryPlugin",
		test(value) {
			return value === ABORT_CONTROLLER;
		},
		parse: {
			sync() {
				return ABORT_CONTROLLER;
			},
			async async() {
				return await Promise.resolve(ABORT_CONTROLLER);
			},
			stream() {
				return ABORT_CONTROLLER;
			}
		},
		serialize() {
			return PROMISE_TO_ABORT_SIGNAL.toString();
		},
		deserialize() {
			throw new Error("seroval-plugins/web/AbortControllerFactoryPlugin cannot be deserialized directly.");
		}
	})],
	test(value) {
		if (typeof AbortSignal === "undefined") return false;
		return value instanceof AbortSignal;
	},
	parse: {
		sync(value, ctx) {
			if (value.aborted) return { reason: ctx.parse(value.reason) };
			return {};
		},
		async async(value, ctx) {
			if (value.aborted) return { reason: await ctx.parse(value.reason) };
			const result = await abortSignalToPromise(value);
			return { reason: await ctx.parse(result) };
		},
		stream(value, ctx) {
			if (value.aborted) return { reason: ctx.parse(value.reason) };
			const promise = abortSignalToPromise(value);
			return {
				factory: ctx.parse(ABORT_CONTROLLER),
				controller: ctx.parse(promise)
			};
		}
	},
	serialize(node, ctx) {
		if (node.reason) return "AbortSignal.abort(" + ctx.serialize(node.reason) + ")";
		if (node.controller && node.factory) return "(" + ctx.serialize(node.factory) + ")(" + ctx.serialize(node.controller) + ").signal";
		return "(new AbortController).signal";
	},
	deserialize(node, ctx) {
		if (node.reason) return AbortSignal.abort(ctx.deserialize(node.reason));
		if (node.controller) {
			const controller = ctx.deserialize(node.controller);
			if (!(controller instanceof Promise)) throw new Error("Expected a Promise source.");
			return PROMISE_TO_ABORT_SIGNAL(controller).signal;
		}
		return new AbortController().signal;
	}
});
function createCustomEventOptions(current) {
	return {
		detail: current.detail,
		bubbles: current.bubbles,
		cancelable: current.cancelable,
		composed: current.composed
	};
}
var CustomEventPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/CustomEvent",
	test(value) {
		if (typeof CustomEvent === "undefined") return false;
		return value instanceof CustomEvent;
	},
	parse: {
		sync(value, ctx) {
			return {
				type: ctx.parse(value.type),
				options: ctx.parse(createCustomEventOptions(value))
			};
		},
		async async(value, ctx) {
			return {
				type: await ctx.parse(value.type),
				options: await ctx.parse(createCustomEventOptions(value))
			};
		},
		stream(value, ctx) {
			return {
				type: ctx.parse(value.type),
				options: ctx.parse(createCustomEventOptions(value))
			};
		}
	},
	serialize(node, ctx) {
		return "new CustomEvent(" + ctx.serialize(node.type) + "," + ctx.serialize(node.options) + ")";
	},
	deserialize(node, ctx) {
		return new CustomEvent(ctx.deserialize(node.type), ctx.deserialize(node.options));
	}
});
var DOMExceptionPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/DOMException",
	test(value) {
		if (typeof DOMException === "undefined") return false;
		return value instanceof DOMException;
	},
	parse: {
		sync(value, ctx) {
			return {
				name: ctx.parse(value.name),
				message: ctx.parse(value.message)
			};
		},
		async async(value, ctx) {
			return {
				name: await ctx.parse(value.name),
				message: await ctx.parse(value.message)
			};
		},
		stream(value, ctx) {
			return {
				name: ctx.parse(value.name),
				message: ctx.parse(value.message)
			};
		}
	},
	serialize(node, ctx) {
		return "new DOMException(" + ctx.serialize(node.message) + "," + ctx.serialize(node.name) + ")";
	},
	deserialize(node, ctx) {
		return new DOMException(ctx.deserialize(node.message), ctx.deserialize(node.name));
	}
});
function createEventOptions(current) {
	return {
		bubbles: current.bubbles,
		cancelable: current.cancelable,
		composed: current.composed
	};
}
var EventPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/Event",
	test(value) {
		if (typeof Event === "undefined") return false;
		return value instanceof Event;
	},
	parse: {
		sync(value, ctx) {
			return {
				type: ctx.parse(value.type),
				options: ctx.parse(createEventOptions(value))
			};
		},
		async async(value, ctx) {
			return {
				type: await ctx.parse(value.type),
				options: await ctx.parse(createEventOptions(value))
			};
		},
		stream(value, ctx) {
			return {
				type: ctx.parse(value.type),
				options: ctx.parse(createEventOptions(value))
			};
		}
	},
	serialize(node, ctx) {
		return "new Event(" + ctx.serialize(node.type) + "," + ctx.serialize(node.options) + ")";
	},
	deserialize(node, ctx) {
		return new Event(ctx.deserialize(node.type), ctx.deserialize(node.options));
	}
});
var FilePlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/File",
	test(value) {
		if (typeof File === "undefined") return false;
		return value instanceof File;
	},
	parse: { async async(value, ctx) {
		return {
			name: await ctx.parse(value.name),
			options: await ctx.parse({
				type: value.type,
				lastModified: value.lastModified
			}),
			buffer: await ctx.parse(await value.arrayBuffer())
		};
	} },
	serialize(node, ctx) {
		return "new File([" + ctx.serialize(node.buffer) + "]," + ctx.serialize(node.name) + "," + ctx.serialize(node.options) + ")";
	},
	deserialize(node, ctx) {
		return new File([ctx.deserialize(node.buffer)], ctx.deserialize(node.name), ctx.deserialize(node.options));
	}
});
function convertFormData(instance) {
	const items = [];
	instance.forEach((value, key) => {
		items.push([key, value]);
	});
	return items;
}
var FORM_DATA_FACTORY = {};
var FORM_DATA_FACTORY_CONSTRUCTOR = (e, f = new FormData(), i = 0, s = e.length, t) => {
	for (; i < s; i++) {
		t = e[i];
		f.append(t[0], t[1]);
	}
	return f;
};
var FormDataPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/FormData",
	extends: [FilePlugin, /* @__PURE__ */ createPlugin({
		tag: "seroval-plugins/web/FormDataFactory",
		test(value) {
			return value === FORM_DATA_FACTORY;
		},
		parse: {
			sync() {
				return FORM_DATA_FACTORY;
			},
			async async() {
				return await Promise.resolve(FORM_DATA_FACTORY);
			},
			stream() {
				return FORM_DATA_FACTORY;
			}
		},
		serialize() {
			return FORM_DATA_FACTORY_CONSTRUCTOR.toString();
		},
		deserialize() {
			return FORM_DATA_FACTORY;
		}
	})],
	test(value) {
		if (typeof FormData === "undefined") return false;
		return value instanceof FormData;
	},
	parse: {
		sync(value, ctx) {
			return {
				factory: ctx.parse(FORM_DATA_FACTORY),
				entries: ctx.parse(convertFormData(value))
			};
		},
		async async(value, ctx) {
			return {
				factory: await ctx.parse(FORM_DATA_FACTORY),
				entries: await ctx.parse(convertFormData(value))
			};
		},
		stream(value, ctx) {
			return {
				factory: ctx.parse(FORM_DATA_FACTORY),
				entries: ctx.parse(convertFormData(value))
			};
		}
	},
	serialize(node, ctx) {
		return "(" + ctx.serialize(node.factory) + ")(" + ctx.serialize(node.entries) + ")";
	},
	deserialize(node, ctx) {
		return FORM_DATA_FACTORY_CONSTRUCTOR(ctx.deserialize(node.entries));
	}
});
function convertHeaders(instance) {
	const items = [];
	instance.forEach((value, key) => {
		items.push([key, value]);
	});
	return items;
}
var HeadersPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/Headers",
	test(value) {
		if (typeof Headers === "undefined") return false;
		return value instanceof Headers;
	},
	parse: {
		sync(value, ctx) {
			return { value: ctx.parse(convertHeaders(value)) };
		},
		async async(value, ctx) {
			return { value: await ctx.parse(convertHeaders(value)) };
		},
		stream(value, ctx) {
			return { value: ctx.parse(convertHeaders(value)) };
		}
	},
	serialize(node, ctx) {
		return "new Headers(" + ctx.serialize(node.value) + ")";
	},
	deserialize(node, ctx) {
		return new Headers(ctx.deserialize(node.value));
	}
});
var READABLE_STREAM_FACTORY = {};
var READABLE_STREAM_FACTORY_CONSTRUCTOR = (stream) => new ReadableStream({ start(controller) {
	stream.on({
		next(value) {
			try {
				controller.enqueue(value);
			} catch (_error) {}
		},
		throw(value) {
			controller.error(value);
		},
		return() {
			try {
				controller.close();
			} catch (_error) {}
		}
	});
} });
var ReadableStreamFactoryPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/ReadableStreamFactory",
	test(value) {
		return value === READABLE_STREAM_FACTORY;
	},
	parse: {
		sync() {
			return READABLE_STREAM_FACTORY;
		},
		async async() {
			return await Promise.resolve(READABLE_STREAM_FACTORY);
		},
		stream() {
			return READABLE_STREAM_FACTORY;
		}
	},
	serialize() {
		return READABLE_STREAM_FACTORY_CONSTRUCTOR.toString();
	},
	deserialize() {
		return READABLE_STREAM_FACTORY;
	}
});
async function drainStream(stream, reader) {
	try {
		while (true) {
			const result = await reader.read();
			if (result.done) {
				stream.return(result.value);
				reader.releaseLock();
				break;
			}
			stream.next(result.value);
		}
	} catch (error) {
		reader.releaseLock();
		stream.throw(error);
	}
}
function cleanupStream(reader) {
	reader.cancel().catch(() => {});
	reader.releaseLock();
}
function toStream(value) {
	const stream = createStream();
	const reader = value.getReader();
	const cleanup = cleanupStream.bind(null, reader);
	drainStream(stream, reader).catch(cleanup);
	return [stream, cleanup];
}
var ReadableStreamPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval/plugins/web/ReadableStream",
	extends: [ReadableStreamFactoryPlugin],
	test(value) {
		if (typeof ReadableStream === "undefined") return false;
		return value instanceof ReadableStream;
	},
	parse: {
		sync(_value, ctx) {
			return {
				factory: ctx.parse(READABLE_STREAM_FACTORY),
				stream: ctx.parse(createStream())
			};
		},
		async async(value, ctx) {
			return {
				factory: await ctx.parse(READABLE_STREAM_FACTORY),
				stream: await ctx.parse(toStream(value)[0])
			};
		},
		stream(value, ctx) {
			const [stream, cleanup] = toStream(value);
			ctx.addCleanup(cleanup);
			return {
				factory: ctx.parse(READABLE_STREAM_FACTORY),
				stream: ctx.parse(stream)
			};
		}
	},
	serialize(node, ctx) {
		return "(" + ctx.serialize(node.factory) + ")(" + ctx.serialize(node.stream) + ")";
	},
	deserialize(node, ctx) {
		const stream = ctx.deserialize(node.stream);
		if (!stream || typeof stream !== "object" || !isStream(stream)) throw new Error("Expected a stream source.");
		return READABLE_STREAM_FACTORY_CONSTRUCTOR(stream);
	}
});
function createRequestOptions(current, body) {
	return {
		body,
		cache: current.cache,
		credentials: current.credentials,
		headers: current.headers,
		integrity: current.integrity,
		keepalive: current.keepalive,
		method: current.method,
		mode: current.mode,
		redirect: current.redirect,
		referrer: current.referrer,
		referrerPolicy: current.referrerPolicy
	};
}
var RequestPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/Request",
	extends: [ReadableStreamPlugin, HeadersPlugin],
	test(value) {
		if (typeof Request === "undefined") return false;
		return value instanceof Request;
	},
	parse: {
		async async(value, ctx) {
			return {
				url: await ctx.parse(value.url),
				options: await ctx.parse(createRequestOptions(value, value.body && !value.bodyUsed ? await value.clone().arrayBuffer() : null))
			};
		},
		stream(value, ctx) {
			return {
				url: ctx.parse(value.url),
				options: ctx.parse(createRequestOptions(value, value.body && !value.bodyUsed ? value.clone().body : null))
			};
		}
	},
	serialize(node, ctx) {
		return "new Request(" + ctx.serialize(node.url) + "," + ctx.serialize(node.options) + ")";
	},
	deserialize(node, ctx) {
		return new Request(ctx.deserialize(node.url), ctx.deserialize(node.options));
	}
});
function createResponseOptions(current) {
	return {
		headers: current.headers,
		status: current.status,
		statusText: current.statusText
	};
}
var ResponsePlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/Response",
	extends: [ReadableStreamPlugin, HeadersPlugin],
	test(value) {
		if (typeof Response === "undefined") return false;
		return value instanceof Response;
	},
	parse: {
		async async(value, ctx) {
			return {
				body: await ctx.parse(value.body && !value.bodyUsed ? await value.clone().arrayBuffer() : null),
				options: await ctx.parse(createResponseOptions(value))
			};
		},
		stream(value, ctx) {
			return {
				body: ctx.parse(value.body && !value.bodyUsed ? value.clone().body : null),
				options: ctx.parse(createResponseOptions(value))
			};
		}
	},
	serialize(node, ctx) {
		return "new Response(" + ctx.serialize(node.body) + "," + ctx.serialize(node.options) + ")";
	},
	deserialize(node, ctx) {
		return new Response(ctx.deserialize(node.body), ctx.deserialize(node.options));
	}
});
var URLPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/URL",
	test(value) {
		if (typeof URL === "undefined") return false;
		return value instanceof URL;
	},
	parse: {
		sync(value, ctx) {
			return { value: ctx.parse(value.href) };
		},
		async async(value, ctx) {
			return { value: await ctx.parse(value.href) };
		},
		stream(value, ctx) {
			return { value: ctx.parse(value.href) };
		}
	},
	serialize(node, ctx) {
		return "new URL(" + ctx.serialize(node.value) + ")";
	},
	deserialize(node, ctx) {
		return new URL(ctx.deserialize(node.value));
	}
});
var URLSearchParamsPlugin = /* @__PURE__ */ createPlugin({
	tag: "seroval-plugins/web/URLSearchParams",
	test(value) {
		if (typeof URLSearchParams === "undefined") return false;
		return value instanceof URLSearchParams;
	},
	parse: {
		sync(value, ctx) {
			return { value: ctx.parse(value.toString()) };
		},
		async async(value, ctx) {
			return { value: await ctx.parse(value.toString()) };
		},
		stream(value, ctx) {
			return { value: ctx.parse(value.toString()) };
		}
	},
	serialize(node, ctx) {
		return "new URLSearchParams(" + ctx.serialize(node.value) + ")";
	},
	deserialize(node, ctx) {
		return new URLSearchParams(ctx.deserialize(node.value));
	}
});
var ChildProperties = /*#__PURE__*/ new Set([
	"innerHTML",
	"textContent",
	"innerText",
	"children"
]);
var COMPOSED_BODY_FRAMING = /*#__PURE__*/ new Set([
	"content-length",
	"content-encoding",
	"transfer-encoding"
]);
function isHttpNavigationTarget(target) {
	try {
		const protocol = new URL(target, "http://base.invalid").protocol;
		return protocol === "http:" || protocol === "https:";
	} catch {
		return false;
	}
}
var STATE = Symbol.for("solid.container-trace-state");
var state = globalThis[STATE] || (globalThis[STATE] = {
	materialized: /* @__PURE__ */ new WeakMap(),
	materializedValues: /* @__PURE__ */ new WeakSet()
});
function setContainerTraceResolver(fn) {
	state.resolveTrace = fn;
}
function setContainerTraceStreamMint(fn) {
	state.streamOf = fn;
}
var TRACE = Symbol.for("solid.container-trace");
function materialize(marker) {
	let value = state.materialized.get(marker.$tr);
	if (value === void 0) {
		value = state.materializeTrace(marker);
		state.materialized.set(marker.$tr, value);
		if (value !== null && typeof value === "object") state.materializedValues.add(value);
	}
	return value;
}
function parseTrace(value, ctx) {
	const trace = value[TRACE];
	const sub = trace.subscribe();
	return {
		a: trace.array ? 1 : 0,
		i: ctx.parse(state.streamOf ? state.streamOf(sub) : sub)
	};
}
var ContainerTracePlugin = {
	tag: "solid/container-trace",
	test(value) {
		return value != null && typeof value === "object" && TRACE in value;
	},
	parse: {
		sync() {
			throw new Error("A reactive container can only be serialized by a streaming serializer.");
		},
		async async(value, ctx) {
			const trace = value[TRACE];
			const sub = trace.subscribe();
			return {
				a: trace.array ? 1 : 0,
				i: await ctx.parse(state.streamOf ? state.streamOf(sub) : sub)
			};
		},
		stream: parseTrace
	},
	serialize(node, ctx) {
		return "{$tr:" + ctx.serialize(node.i) + ",$ta:" + node.a + "}";
	},
	deserialize(node, ctx) {
		const marker = {
			$tr: ctx.deserialize(node.i),
			$ta: node.a
		};
		return state.materializeTrace ? materialize(marker) : marker;
	}
};
setContainerTraceStreamMint((iterable) => {
	const stream = createStream();
	(async () => {
		try {
			for await (const value of iterable) stream.next(value);
			stream.return(void 0);
		} catch (error) {
			stream.throw(error);
		}
	})();
	return stream;
});
var DEFAULT_WEB_PLUGINS = Object.freeze([
	AbortSignalPlugin,
	CustomEventPlugin,
	DOMExceptionPlugin,
	EventPlugin,
	FormDataPlugin,
	HeadersPlugin,
	ReadableStreamPlugin,
	RequestPlugin,
	ResponsePlugin,
	URLSearchParamsPlugin,
	URLPlugin,
	ContainerTracePlugin
]);
function resolveSerializerPlugins(customPlugins) {
	return customPlugins ? [...customPlugins, ...DEFAULT_WEB_PLUGINS] : [...DEFAULT_WEB_PLUGINS];
}
Feature.RegExp;
var DEFAULT_DISABLED_FEATURES = Feature.AggregateError | Feature.BigIntTypedArray;
var serializeOnlyDisabledFeatures = (serializeErrorStacks) => (serializeErrorStacks === void 0 ? false : serializeErrorStacks) ? 0 : Feature.ErrorPrototypeStack;
var HYDRATION_GLOBAL = "_$HY.r";
function createSerializer(options) {
	return new Serializer({
		...options,
		plugins: resolveSerializerPlugins(options.plugins),
		disabledFeatures: (options.disabledFeatures === void 0 ? DEFAULT_DISABLED_FEATURES : options.disabledFeatures) | serializeOnlyDisabledFeatures(options.serializeErrorStacks)
	});
}
function createHydrationSerializer({ onData, onDone, scopeId, onError, plugins }) {
	return createSerializer({
		scopeId,
		plugins,
		globalIdentifier: HYDRATION_GLOBAL,
		onData,
		onDone,
		onError
	});
}
function getLocalHeaderScript(id) {
	return getCrossReferenceHeader(id) + ";";
}
var REVALIDATE_HEADER = "X-Revalidate";
new RegExp(`(?:^|;\\s*)flash=([^;]+)`);
var ERROR_HEADER = "X-Server-Function-Error";
var BODY_FORMAT_HEADER = "X-Server-Function-Format";
var REDIRECT_HEADER = "X-Server-Function-Redirect";
var SINGLE_FLIGHT_HEADER = "X-Single-Flight";
var HEAD_ELIGIBLE_TAGS = /* @__PURE__ */ new Set([
	"title",
	"meta",
	"link",
	"style",
	"script",
	"base"
]);
var HEAD_ATTR_NAME = /^[a-zA-Z_][a-zA-Z0-9_:.-]*$/;
var RESOURCE_LINK_RELS = /* @__PURE__ */ new Set([
	"preload",
	"modulepreload",
	"prefetch",
	"preconnect",
	"dns-prefetch",
	"stylesheet"
]);
var RESOURCE_QUALIFIERS = [
	"as",
	"crossorigin",
	"type",
	"media",
	"imagesrcset",
	"imagesizes"
];
var STYLESHEET_FETCH_META = /* @__PURE__ */ new Set([
	"crossorigin",
	"integrity",
	"referrerpolicy",
	"fetchpriority"
]);
function evalHeadValue(v) {
	return typeof v === "function" ? v() : v;
}
function evalHeadProps(props, presets) {
	const out = {};
	for (const name in props) out[name] = presets && name in presets ? presets[name] : evalHeadValue(props[name]);
	return out;
}
function classifyHeadTag(desc) {
	const tag = desc.tag;
	if (tag === "link") {
		const rel = evalHeadValue(desc.props && desc.props.rel);
		return {
			resource: RESOURCE_LINK_RELS.has(rel),
			rel
		};
	}
	if (tag === "style") return { resource: !!(desc.props && "href" in desc.props) };
	if (tag === "script") return { resource: !!(desc.props && "src" in desc.props) };
	return { resource: false };
}
function qualifierValue(name, value) {
	if (value == null || value === false) return null;
	if (name === "imagesrcset" || name === "imagesizes") return typeof value === "string" && value !== "" ? value : null;
	const v = value === true ? "" : String(value);
	if (name === "as") return asciiLowerCase(v);
	if (name !== "crossorigin") return v;
	return v.length === 15 && v.toLowerCase() === "use-credentials" ? v.toLowerCase() : "anonymous";
}
function asciiLowerCase(value) {
	return value.replace(/[A-Z]/g, (c) => String.fromCharCode(c.charCodeAt(0) + 32));
}
function resourceIdentity(tag, props) {
	const url = String(props.href || props.src || "");
	let id = "res:" + tag + ":" + (props.rel || "") + ":" + url.length + ":" + url;
	for (let i = 0; i < RESOURCE_QUALIFIERS.length; i++) {
		const q = RESOURCE_QUALIFIERS[i];
		const value = qualifierValue(q, props[q]);
		if (value !== null) id += ":" + q + "=" + value.length + ":" + value;
	}
	return id;
}
function replaceableIdentity(tag, props, key, unique) {
	if (tag === "title") return "title";
	if (tag === "base") return "base";
	if (tag === "meta" && props.charset != null) return "charset";
	if (key != null) return tag + ":key:" + key;
	if (tag === "meta") {
		for (const ns of [
			"name",
			"property",
			"http-equiv"
		]) if (props[ns] != null) return "meta:" + ns + ":" + props[ns] + (props.media != null ? ":media=" + props.media : "");
		return unique;
	}
	if (tag === "link") {
		const rel = props.rel || "";
		if (rel === "icon" || rel === "apple-touch-icon") return "link:" + rel + (props.sizes != null ? ":sizes=" + props.sizes : "") + (props.type != null ? ":type=" + props.type : "");
		return "link:" + rel + ":" + (props.href || "");
	}
	return unique;
}
function resolveHead(groups) {
	const winners = /* @__PURE__ */ new Map();
	const sorted = groups.slice().sort((a, b) => a.seq - b.seq);
	for (let i = 0; i < sorted.length; i++) {
		const group = sorted[i];
		const byIdentity = /* @__PURE__ */ new Map();
		for (let j = 0; j < group.tags.length; j++) {
			const t = group.tags[j];
			let list = byIdentity.get(t.identity);
			if (!list) byIdentity.set(t.identity, list = []);
			list.push(t);
		}
		for (const [identity, tags] of byIdentity) if (identity === "title") winners.set(identity, {
			seq: group.seq,
			tags: [tags[tags.length - 1]]
		});
		else winners.set(identity, {
			seq: group.seq,
			tags
		});
	}
	return winners;
}
function joinAssetPath$1(base, file) {
	if (/^(?:[a-z][a-z0-9+.-]*:)?\/\//i.test(file)) return file;
	if (typeof base !== "string" || !base) base = "/";
	if (base[base.length - 1] !== "/") base += "/";
	return base + (file[0] === "/" ? file.slice(1) : file);
}
function resolveAssets(moduleUrl, manifest) {
	if (!manifest) return null;
	const base = manifest._base;
	if (!manifest[moduleUrl]) return null;
	const css = [];
	const js = [];
	let preloads;
	const visited = /* @__PURE__ */ new Set();
	const walk = (key) => {
		if (visited.has(key)) return;
		visited.add(key);
		const e = manifest[key];
		if (!e) return;
		js.push(joinAssetPath$1(base, e.file));
		if (e.css) for (let i = 0; i < e.css.length; i++) css.push(joinAssetPath$1(base, e.css[i]));
		if (e.preloads) for (let i = 0; i < e.preloads.length; i++) {
			const link = e.preloads[i];
			const href = link && typeof link.href === "string" && link.href;
			const srcset = link && typeof link.imagesrcset === "string" && link.imagesrcset;
			if (!href && !srcset) continue;
			if (!preloads) preloads = [];
			if (href) preloads.push({
				...link,
				href: joinAssetPath$1(base, href)
			});
			else {
				const { href: bad, ...rest } = link;
				preloads.push(rest);
			}
		}
		if (e.imports) for (let i = 0; i < e.imports.length; i++) walk(e.imports[i]);
	};
	walk(moduleUrl);
	const assets = {
		js,
		css
	};
	if (preloads) assets.preloads = preloads;
	return assets;
}
function registerEntryAssets(manifest) {
	if (!manifest || typeof manifest === "function" || typeof manifest.resolve === "function") return;
	const ctx = sharedConfig.context;
	if (!ctx?.registerAsset) return;
	for (const key in manifest) if (manifest[key].isEntry) {
		const assets = resolveAssets(key, manifest);
		if (assets) {
			if (assets.preloads) for (let i = 0; i < assets.preloads.length; i++) ctx.registerAsset("preload", assets.preloads[i]);
			for (let i = 0; i < assets.css.length; i++) ctx.registerAsset("style", assets.css[i]);
			for (let i = 1; i < assets.js.length; i++) ctx.registerAsset("module", assets.js[i]);
		}
		return;
	}
}
function createAssetTracking() {
	const boundaryModules = /* @__PURE__ */ new Map();
	const boundaryStyles = /* @__PURE__ */ new Map();
	const emittedAssets = /* @__PURE__ */ new Set();
	const inlineStyles = /* @__PURE__ */ new Map();
	let currentBoundaryId = null;
	return {
		boundaryModules,
		boundaryStyles,
		emittedAssets,
		inlineStyles,
		preloadLinks: null,
		registerInlineStyle(desc) {
			let entry = inlineStyles.get(desc.id);
			if (!entry) {
				entry = {
					id: desc.id,
					content: desc.content || "",
					attrs: desc.attrs,
					emitted: false
				};
				inlineStyles.set(desc.id, entry);
			}
			if (currentBoundaryId) {
				let styles = boundaryStyles.get(currentBoundaryId);
				if (!styles) {
					styles = /* @__PURE__ */ new Set();
					boundaryStyles.set(currentBoundaryId, styles);
				}
				styles.add(entry);
			}
			return entry;
		},
		get currentBoundaryId() {
			return currentBoundaryId;
		},
		set currentBoundaryId(v) {
			currentBoundaryId = v;
		},
		registerModule(key, entryUrl) {
			const id = currentBoundaryId || "";
			let map = boundaryModules.get(id);
			if (!map) {
				map = {};
				boundaryModules.set(id, map);
			}
			map[key] = entryUrl;
		},
		getBoundaryModules(id) {
			return boundaryModules.get(id) || null;
		},
		getBoundaryStyles(id) {
			return boundaryStyles.get(id) || null;
		}
	};
}
function warnUnresolvedModuleAssets(moduleUrl, warned) {
	if (warned.has(moduleUrl)) return;
	warned.add(moduleUrl);
	console.error(`Asset manifest returned no client assets for module "${moduleUrl}". If this module is a server-rendered lazy() component, its entry will be missing from the serialized hydration asset map, the client will be unable to preload it, and hydration will fail with 'lazy() module "…" was not preloaded before hydration'. This means the integration's asset resolver (dev manifest bridge or build client manifest) failed to answer for this module — check the integration's server logs, restart the dev server, or verify the module is included in the client build.`);
}
function guardResolvedAssets(moduleUrl, result, warned) {
	if (result && typeof result.then === "function") return result.then((assets) => {
		if (!assets || !assets.js || !assets.js.length) warnUnresolvedModuleAssets(moduleUrl, warned);
		return assets;
	});
	if (!result || !result.js || !result.js.length) warnUnresolvedModuleAssets(moduleUrl, warned);
	return result;
}
function applyAssetTracking(context, tracking, manifest, noScripts) {
	const warned = /* @__PURE__ */ new Set();
	const guard = noScripts ? (resolve) => resolve : (resolve) => (moduleUrl) => guardResolvedAssets(moduleUrl, resolve(moduleUrl), warned);
	Object.defineProperty(context, "_currentBoundaryId", {
		get() {
			return tracking.currentBoundaryId;
		},
		set(v) {
			tracking.currentBoundaryId = v;
		},
		configurable: true,
		enumerable: true
	});
	context.registerModule = tracking.registerModule;
	context.getBoundaryModules = tracking.getBoundaryModules;
	if (typeof manifest === "function") context.resolveAssets = guard(manifest);
	else if (manifest && typeof manifest.resolve === "function") {
		context.resolveAssets = guard((key) => manifest.resolve(key));
		if (typeof manifest.resolveSync === "function") context.resolveAssetsSync = (key) => manifest.resolveSync(key);
	} else if (manifest) {
		const resolve = (moduleUrl) => resolveAssets(moduleUrl, manifest);
		context.resolveAssets = guard(resolve);
		context.resolveAssetsSync = resolve;
	}
}
function isCssUrl(url) {
	const q = url.search(/[?#]/);
	return (q === -1 ? url : url.slice(0, q)).endsWith(".css");
}
var RESPONSIVE_ATTRIBUTES = ["imagesrcset", "imagesizes"];
function isSetAttr(value) {
	return value != null && value !== false && value !== "";
}
var PRELOAD_LINK_ATTRIBUTES = [
	"type",
	"crossorigin",
	"integrity",
	"referrerpolicy",
	"fetchpriority",
	"media",
	"imagesrcset",
	"imagesizes"
];
function registerPreloadLink(tracking, headRegistry, link, nonce) {
	if (!link || typeof link !== "object") return null;
	if (typeof link.as !== "string") return null;
	const as = asciiLowerCase(link.as);
	let destination = null;
	switch (as) {
		case "script":
		case "style":
			destination = as;
			break;
		case "fetch":
		case "font":
		case "image":
		case "track": break;
		default: return null;
	}
	const responsive = as === "image";
	let srcset = null;
	let sizes = null;
	if (responsive) for (const name of RESPONSIVE_ATTRIBUTES) {
		const value = link[name];
		if (!isSetAttr(value)) continue;
		if (typeof value !== "string") continue;
		if (name === "imagesrcset") srcset = value;
		else sizes = value;
	}
	const href = typeof link.href === "string" && link.href ? link.href : null;
	if (!href && !srcset) return null;
	const props = { rel: "preload" };
	if (href) props.href = href;
	props.as = as;
	for (let i = 0; i < PRELOAD_LINK_ATTRIBUTES.length; i++) {
		const name = PRELOAD_LINK_ATTRIBUTES[i];
		if (RESPONSIVE_ATTRIBUTES.indexOf(name) !== -1) {
			const value = name === "imagesrcset" ? srcset : sizes;
			if (value !== null) props[name] = value;
			continue;
		}
		const value = link[name];
		if (value == null || value === false) continue;
		props[name] = value === true ? "" : String(value);
	}
	const identity = resourceIdentity("link", props);
	if (headRegistry.resources.has(identity)) return null;
	const attrs = headAttrRecord(props);
	const nonceValue = destination && nonce && nonce[destination];
	if (typeof nonceValue === "string" && nonceValue) attrs.nonce = nonceValue;
	const entry = {
		href: props.href,
		attrs,
		attrHtml: renderHeadAttrHtml(props) + nonceAttr(nonce, destination)
	};
	headRegistry.resources.add(identity);
	let links = tracking.preloadLinks;
	if (!links) tracking.preloadLinks = links = [];
	links.push(entry);
	return entry;
}
function createHeadRegistry() {
	return {
		pending: [],
		committed: [],
		seq: 0,
		uniq: 0,
		resources: /* @__PURE__ */ new Set(),
		eagerHtml: "",
		flushed: null,
		shellFlushed: false,
		parkedResources: []
	};
}
function registerHeadTags(registry, context, tracking, emitResource, nonce, tags) {
	const boundary = context._currentBoundaryId || "";
	if (typeof tags === "function") {
		registry.pending.push({
			boundary,
			list: tags,
			resource: (desc, rel) => emitHeadResource(registry, context, tracking, emitResource, nonce, desc, rel)
		});
		return;
	}
	if (!Array.isArray(tags)) tags = [tags];
	const probe = sharedConfig.context && sharedConfig.context._loadingPhase;
	let replaceable = null;
	for (let i = 0; i < tags.length; i++) {
		const desc = tags[i];
		if (!desc || !HEAD_ELIGIBLE_TAGS.has(desc.tag)) continue;
		const cls = classifyHeadTag(desc);
		if (probe && !cls.resource) try {
			evalHeadProps(desc.props || {}, cls.rel !== void 0 ? { rel: cls.rel } : void 0);
			evalHeadValue(desc.key);
		} catch (err) {
			if (ssrHandleError(err)) throw err;
		}
		if (cls.resource) emitHeadResource(registry, context, tracking, emitResource, nonce, desc, cls.rel);
		else (replaceable || (replaceable = [])).push(cls.rel !== void 0 ? {
			tag: desc.tag,
			props: desc.props,
			key: desc.key,
			rel: cls.rel
		} : desc);
	}
	if (replaceable) registry.pending.push({
		boundary,
		tags: replaceable
	});
}
function headShellReady(registry, block) {
	let ready = true;
	const pends = (err) => {
		const source = ssrHandleError(err, true);
		if (!source) return false;
		block(source);
		ready = false;
		return true;
	};
	const parked = registry.parkedResources;
	for (let i = parked.length - 1; i >= 0; i--) {
		const { desc, rel, emit } = parked[i];
		try {
			evalHeadProps(desc.props || {}, rel !== void 0 ? { rel } : void 0);
		} catch (err) {
			if (pends(err)) continue;
			parked.splice(i, 1);
			continue;
		}
		parked.splice(i, 1);
		emit();
	}
	for (let i = 0; i < registry.pending.length; i++) {
		const reg = registry.pending[i];
		if (reg.boundary !== "" || reg.list) continue;
		for (let j = 0; j < reg.tags.length; j++) {
			const desc = reg.tags[j];
			try {
				evalHeadProps(desc.props || {}, desc.rel !== void 0 ? { rel: desc.rel } : void 0);
				evalHeadValue(desc.key);
			} catch (err) {
				pends(err);
			}
		}
	}
	return ready;
}
function emitHeadResource(registry, context, tracking, emitResource, nonce, desc, rel) {
	let props;
	try {
		props = evalHeadProps(desc.props || {}, rel !== void 0 ? { rel } : void 0);
	} catch (err) {
		const loadingPhase = sharedConfig.context && sharedConfig.context._loadingPhase;
		if (loadingPhase && ssrHandleError(err)) throw err;
		if (!loadingPhase && !context._currentBoundaryId && !registry.shellFlushed && typeof context.block === "function" && ssrHandleError(err, true)) {
			registry.parkedResources.push({
				desc,
				rel,
				emit: () => emitHeadResource(registry, context, tracking, emitResource, nonce, desc, rel)
			});
			return;
		}
		return;
	}
	const identity = resourceIdentity(desc.tag, props);
	if (registry.resources.has(identity)) return;
	registry.resources.add(identity);
	if (desc.tag === "link" && (rel === "stylesheet" || rel === "modulepreload")) {
		let plain = true;
		let gateable = rel === "stylesheet";
		for (const name in props) {
			if (name === "rel" || name === "href") continue;
			plain = false;
			if (!STYLESHEET_FETCH_META.has(name)) gateable = false;
		}
		if (plain && props.href != null) {
			const isCss = isCssUrl(props.href);
			if (rel === "stylesheet" ? isCss : !isCss) {
				context.registerAsset(rel === "stylesheet" ? "style" : "module", props.href);
				return;
			}
		}
		if (gateable && props.href != null) {
			const attrHtml = renderHeadAttrHtml(props);
			const entry = {
				href: props.href,
				attrHtml,
				attrs: headAttrRecord(props)
			};
			if (tracking.currentBoundaryId) {
				let styles = tracking.boundaryStyles.get(tracking.currentBoundaryId);
				if (!styles) tracking.boundaryStyles.set(tracking.currentBoundaryId, styles = /* @__PURE__ */ new Set());
				styles.add(entry);
			}
			const markup = `<link${attrHtml}${nonceAttr(nonce, "style")}>`;
			if (emitResource) emitResource(markup, entry);
			else {
				registry.eagerHtml += markup;
				entry.emitted = true;
			}
			return;
		}
	}
	const url = props.href || props.src;
	if (url != null && tracking.emittedAssets.has(url)) return;
	const markup = renderHeadTagMarkup(desc.tag, props, null, nonce);
	if (emitResource) emitResource(markup);
	else registry.eagerHtml += markup;
}
function commitHeadBoundary(registry, boundary, isPendingFragment) {
	const keep = [];
	const groups = [];
	for (let i = 0; i < registry.pending.length; i++) {
		const reg = registry.pending[i];
		if (!(boundary === "" ? !(isPendingFragment && reg.boundary !== "" && isPendingFragment(reg.boundary)) : reg.boundary === boundary)) {
			keep.push(reg);
			continue;
		}
		let descs = reg.tags;
		if (reg.list) {
			let resolved;
			try {
				resolved = reg.list();
			} catch (err) {
				continue;
			}
			if (!Array.isArray(resolved)) resolved = [resolved];
			descs = [];
			for (let j = 0; j < resolved.length; j++) {
				const desc = resolved[j];
				if (!desc || !HEAD_ELIGIBLE_TAGS.has(desc.tag)) continue;
				const cls = classifyHeadTag(desc);
				if (cls.resource) reg.resource(desc, cls.rel);
				else descs.push(cls.rel !== void 0 ? {
					tag: desc.tag,
					props: desc.props,
					key: desc.key,
					rel: cls.rel
				} : desc);
			}
		}
		const tags = [];
		for (let j = 0; j < descs.length; j++) {
			const desc = descs[j];
			let props, key;
			try {
				props = evalHeadProps(desc.props || {}, desc.rel !== void 0 ? { rel: desc.rel } : void 0);
				key = evalHeadValue(desc.key);
			} catch (err) {
				continue;
			}
			const identity = replaceableIdentity(desc.tag, props, key, "u:" + registry.uniq++);
			if ((identity === "base" || identity === "charset") && registry.shellFlushed) continue;
			tags.push({
				tag: desc.tag,
				props,
				identity
			});
		}
		if (tags.length) groups.push({
			seq: registry.seq++,
			tags
		});
	}
	registry.pending = keep;
	for (let i = 0; i < groups.length; i++) registry.committed.push(groups[i]);
	return groups;
}
function adoptHeadBoundary(registry, childKey, parentKey) {
	for (let i = 0; i < registry.pending.length; i++) if (registry.pending[i].boundary === childKey) registry.pending[i].boundary = parentKey;
}
function dropHeadBoundary(registry, boundary) {
	registry.pending = registry.pending.filter((reg) => reg.boundary !== boundary);
}
function headGroupSignature(winner) {
	let sig = "" + winner.seq;
	for (let i = 0; i < winner.tags.length; i++) {
		const t = winner.tags[i];
		sig += "|" + t.tag + JSON.stringify(t.props);
	}
	return sig;
}
function renderShellHead(registry, nonce, isPendingFragment, noScripts) {
	commitHeadBoundary(registry, "", isPendingFragment);
	registry.shellFlushed = true;
	const winners = resolveHead(registry.committed);
	registry.flushed = /* @__PURE__ */ new Map();
	let prelude = "";
	let links = "";
	let metas = "";
	let others = "";
	let scripts = "";
	let title = null;
	for (const [identity, winner] of winners) {
		registry.flushed.set(identity, headGroupSignature(winner));
		if (identity === "title") {
			const children = winner.tags[0].props.children;
			title = children == null ? "" : String(children);
			continue;
		}
		for (let i = 0; i < winner.tags.length; i++) {
			const t = winner.tags[i];
			const markup = renderHeadTagMarkup(t.tag, t.props, identity, nonce);
			if (identity === "charset" || identity === "base") prelude += markup;
			else if (t.tag === "link" || t.tag === "style") links += markup;
			else if (t.tag === "meta") metas += markup;
			else if (t.tag === "script") scripts += markup;
			else others += markup;
		}
	}
	return {
		prelude,
		html: registry.eagerHtml + links + metas + others + scripts,
		title,
		noScripts
	};
}
function flushHeadFragment(registry, boundary, nonce) {
	const groups = commitHeadBoundary(registry, boundary);
	if (!groups.length) return null;
	const winners = resolveHead(registry.committed);
	const affected = /* @__PURE__ */ new Set();
	for (let i = 0; i < groups.length; i++) for (let j = 0; j < groups[i].tags.length; j++) affected.add(groups[i].tags[j].identity);
	const ops = [];
	for (const identity of affected) {
		const winner = winners.get(identity);
		const sig = headGroupSignature(winner);
		if (registry.flushed.get(identity) === sig) continue;
		const existed = registry.flushed.has(identity);
		registry.flushed.set(identity, sig);
		if (identity === "title") {
			const children = winner.tags[0].props.children;
			ops.push(["t", children == null ? "" : String(children)]);
			continue;
		}
		if (existed) ops.push(["r", identity]);
		for (let i = 0; i < winner.tags.length; i++) {
			const t = winner.tags[i];
			const attrs = {};
			for (const name in t.props) {
				if (name === "children" || name === "ref" || name.slice(0, 2) === "on") continue;
				if (!HEAD_ATTR_NAME.test(name)) continue;
				const v = t.props[name];
				if (v == null || v === false) continue;
				attrs[name] = v === true ? "" : String(v);
			}
			if (nonce && !hasNonceProp(t.props)) {
				const destination = nonceDestination(t.tag, t.props);
				const value = destination && nonce[destination];
				if (value) attrs.nonce = String(value);
			}
			const children = t.props.children;
			ops.push([
				"a",
				identity,
				t.tag,
				attrs,
				children == null ? null : String(children)
			]);
		}
	}
	return ops.length ? ops : null;
}
function normalizeNonce(nonce) {
	if (nonce == null) return void 0;
	if (typeof nonce === "string") {
		const attr = nonce ? ` nonce="${escape(nonce, true)}"` : "";
		return {
			script: nonce,
			style: nonce,
			scriptAttr: attr,
			styleAttr: attr
		};
	}
	const script = nonce.script;
	const style = nonce.style;
	if (!script && !style) return void 0;
	return {
		script,
		style,
		scriptAttr: typeof script === "string" && script ? ` nonce="${escape(script, true)}"` : "",
		styleAttr: typeof style === "string" && style ? ` nonce="${escape(style, true)}"` : ""
	};
}
function destinationNonce(nonce, destination) {
	if (nonce == null) return void 0;
	if (typeof nonce === "string") return nonce || void 0;
	const value = nonce[destination];
	return typeof value === "string" && value ? value : void 0;
}
function scriptNonce(nonce) {
	return destinationNonce(nonce, "script");
}
function hasNonceProp(props) {
	for (const name in props) if (name.length === 5 && asciiLowerCase(name) === "nonce" && props[name] != null) return true;
	return false;
}
function nonceDestination(tag, props) {
	if (tag === "script") return "script";
	if (tag === "style") return "style";
	if (tag !== "link") return null;
	const rel = props.rel;
	if (typeof rel !== "string") return null;
	const rels = asciiLowerCase(rel).split(/[\t\n\f\r ]+/);
	if (rels.includes("stylesheet")) return "style";
	const isModulePreload = rels.includes("modulepreload");
	if (!isModulePreload && !rels.includes("preload")) return null;
	const as = typeof props.as === "string" ? asciiLowerCase(props.as) : "";
	if (as === "style") return "style";
	if (as === "script") return "script";
	if (!isModulePreload) return null;
	switch (as) {
		case "fetch":
		case "font":
		case "image":
		case "json":
		case "text":
		case "track": return null;
		default: return "script";
	}
}
function nonceAttr(nonce, destination) {
	if (!nonce || !destination) return "";
	return destination === "script" ? nonce.scriptAttr : nonce.styleAttr;
}
function renderHeadAttrHtml(props) {
	let attrs = "";
	for (const name in props) {
		if (name === "children" || name === "ref" || name.slice(0, 2) === "on") continue;
		if (!HEAD_ATTR_NAME.test(name)) continue;
		const v = props[name];
		if (v == null || v === false) continue;
		attrs += v === true ? ` ${name}` : ` ${name}="${escape(String(v), true)}"`;
	}
	return attrs;
}
function headAttrRecord(props, skipRelHref) {
	let attrs = null;
	for (const name in props) {
		if (name === "children" || name === "ref" || name.slice(0, 2) === "on") continue;
		if (name === "rel" || name === "href") continue;
		if (!HEAD_ATTR_NAME.test(name)) continue;
		const v = props[name];
		if (v == null || v === false) continue;
		(attrs || (attrs = {}))[name] = v === true ? "" : String(v);
	}
	return attrs;
}
function renderHeadTagMarkup(tag, props, identity, nonce) {
	let attrs = renderHeadAttrHtml(props);
	if (identity != null) attrs += ` data-dh="${escape(identity, true)}"`;
	if (nonce && !hasNonceProp(props)) attrs += nonceAttr(nonce, nonceDestination(tag, props));
	if (tag === "meta" || tag === "link" || tag === "base") return `<${tag}${attrs}>`;
	let body = props.children == null ? "" : String(props.children);
	if (tag === "script") body = body.replace(/<\/(script)/gi, "<\\/$1");
	else if (tag === "style") body = escapeStyleContent(body);
	else body = escape(body);
	return `<${tag}${attrs}>${body}</${tag}>`;
}
var VOID_ELEMENTS = /^(?:area|base|br|col|embed|hr|img|input|keygen|link|menuitem|meta|param|source|track|wbr)$/i;
var REPLACE_SCRIPT = `function $df(e){return _$HY.f?_$HY.f(e):$dfr(e)}function $dfr(e,n,o,t){if(!(n=document.getElementById(e)))return 0;if(!(o=document.getElementById("pl-"+e)))return(_$HY.dq=_$HY.dq||{})[e]=1,0;for(;o&&(8!==o.nodeType||o.nodeValue!=="pl-"+e);)t=o.nextSibling,o.remove(),o=t;t=o.parentNode,o.replaceWith(n.content),n.remove(),(_$HY.v=_$HY.v||{})[e]=1,_$HY.fe(e,t),_$HY.hp&&_$HY.hp[e]&&($dh(_$HY.hp[e]),delete _$HY.hp[e]),$dfd();return 1}function $dfl(e,o,n){if(!(o=document.getElementById("pl-"+e)))return(_$HY.dlq=_$HY.dlq||{})[e]=1,0;if(o._$fl)return 1;for(n=o.nextSibling;n;){if(8===n.nodeType&&n.nodeValue==="pl-"+e){o.parentNode&&o.parentNode.insertBefore(o.content.cloneNode(!0),n),o._$fl=1,$dfd();return 1}n=n.nextSibling}return 0}function $dflj(e,i){for(i=0;i<e.length;i++)$dfl(e[i])}function $dfd(e,i){if(e=_$HY.dq){_$HY.dq=0;for(i in e)$df(i)}if(e=_$HY.dlq){_$HY.dlq=0;for(i in e)$dfl(i)}}function $dfs(e,c,d){(_$HY.sc=_$HY.sc||{})[e]=c,d&&((_$HY.sd=_$HY.sd||{})[e]=1)}function $dfg(e,g,i,k){if(!(g=_$HY.sg&&_$HY.sg[e]))return;for(i=0;i<g.length;i++)if(_$HY.sc&&_$HY.sc[g[i]]>0)return;for(i=0;i<g.length;i++)k=g[i],delete _$HY.sg[k],$df(k)}function $dfc(e){if(--_$HY.sc[e]<=0){delete _$HY.sc[e],_$HY.sg&&_$HY.sg[e]?$dfg(e):!(_$HY.sd&&_$HY.sd[e])&&$df(e);_$HY.sd&&delete _$HY.sd[e]}}function $dfj(e,i,n){for(i=0;i<e.length;i++)if(_$HY.sc&&_$HY.sc[e[i]]>0){for(n=0;n<e.length;n++)(_$HY.sg=_$HY.sg||{})[e[n]]=e;return}for(i=0;i<e.length;i++)$df(e[i])}`;
var HEAD_SCRIPT = `function $dha(o,i,e,n){for(i=0;i<o.length;i++)e=o[i],"t"==e[0]?((n=document.querySelector("title"))?n.hasAttribute("data-dh")||n.setAttribute("data-dhf",n.textContent):(n=document.createElement("title"),document.head.appendChild(n)),n.textContent=e[1],n.setAttribute("data-dh","title")):"r"==e[0]?$dhr(e[1]):(n=document.createElement(e[2]),Object.keys(e[3]).forEach(function(a){n.setAttribute(a,e[3][a])}),null!=e[4]&&(n.textContent=e[4]),n.setAttribute("data-dh",e[1]),document.head.appendChild(n))}function $dhr(v,l,i){for(l=document.head.querySelectorAll("[data-dh]"),i=0;i<l.length;i++)l[i].getAttribute("data-dh")==v&&l[i].remove()}function $dh(o){_$HY.h?_$HY.h(o):$dha(o)}`;
function renderToStream(code, options = {}) {
	let { onCompleteShell, onCompleteAll, renderId = "", noScripts, manifest, onHead } = options;
	const nonce = normalizeNonce(options.nonce);
	const requestEvent = peekRequestEvent();
	let dispose;
	let dead = false;
	const abandon = () => {
		if (dead) return;
		dead = true;
		completed = true;
		buffer = { write() {} };
		writable = { end() {} };
		if (dispose) {
			const d = dispose;
			dispose = () => {};
			d();
		}
	};
	const failRender = (err) => {
		try {
			options.onError ? options.onError(err) : console.error(err);
		} catch (_) {}
		abandon();
	};
	const coalesceWrites = (writeRaw, endRaw) => {
		let buf = "";
		let scheduled = false;
		const flush = () => {
			scheduled = false;
			if (!buf) return;
			const out = buf;
			buf = "";
			writeRaw(out);
		};
		return {
			write(payload) {
				buf += payload;
				if (buf.length >= 16384) return flush();
				if (!scheduled) {
					scheduled = true;
					deferFlush(flush);
				}
			},
			flush,
			end() {
				flush();
				endRaw();
			}
		};
	};
	const guardSink = (w) => ({
		write(payload) {
			if (dead) return;
			try {
				w.write(payload);
			} catch (_) {
				abandon();
			}
		},
		end() {
			if (dead) return;
			try {
				w.end();
			} catch (_) {
				abandon();
			}
		}
	});
	const blockingPromises = /* @__PURE__ */ new Set();
	const canBatchStubs = !options.serializer && !options.sink;
	let stubBatch = null;
	const STUB_BATCH_KEY = "$B";
	const flushStubBatch = () => {
		if (!stubBatch) return;
		const batch = stubBatch;
		stubBatch = null;
		if (batch.size === 1) {
			const [id, p] = batch.entries().next().value;
			serializer.write(id, p);
			return;
		}
		const obj = {};
		for (const [id, p] of batch) obj[id] = p;
		serializer.write(STUB_BATCH_KEY, obj);
		pushTask(`(b=>{for(var k in b)_$HY.r[k]=b[k];delete _$HY.r["${STUB_BATCH_KEY}"]})(_$HY.r["${STUB_BATCH_KEY}"])`);
	};
	let headerEmitted = false;
	const pushTask = (task) => {
		if (noScripts) return;
		if (!headerEmitted) {
			headerEmitted = true;
			tasks += getLocalHeaderScript(renderId);
		}
		tasks += task + ";";
		if (!timer && firstFlushed) {
			timer = true;
			queue(() => queue(writeTasks));
		}
	};
	const onDone = () => {
		writeTasks();
		lastBlockingSize = blockingPromises.size;
		doShell();
		onCompleteAll && onCompleteAll({ write(v) {
			!completed && buffer.write(v);
		} });
		writable && writable.end();
		completed = true;
		if (firstFlushed) dispose();
	};
	const sink = {
		data(payload) {
			pushTask(payload);
		},
		fragment(key, value, meta) {
			const deferActivation = !!meta.revealGroup;
			const styles = meta.styles;
			for (let i = 0; i < styles.inline.length; i++) buffer.write(renderInlineStyle(styles.inline[i], nonce));
			if (styles.links.length) {
				const styleAttr = nonceAttr(nonce, "style");
				emitTask(`$dfs("${key}",${styles.links.length},${deferActivation ? 1 : 0})`);
				writeTasks();
				for (const entry of styles.links) buffer.write(typeof entry === "string" ? `<link rel="stylesheet" href="${entry}"${styleAttr} onload="$dfc('${key}')" onerror="$dfc('${key}')">` : `<link${entry.attrHtml}${styleAttr} onload="$dfc('${key}')" onerror="$dfc('${key}')">`);
				buffer.write(`<template id="${key}">${value}</template>`);
			} else {
				buffer.write(`<template id="${key}">${value}</template>`);
				if (!deferActivation) emitTask(`$df("${key}")`);
			}
		},
		reveal(keys, meta) {
			emitTask(`${meta.fallback ? "$dflj" : "$dfj"}(${JSON.stringify(keys)})`);
		},
		asset(type, value) {
			if (type === "module") buffer.write(`<link rel="modulepreload" href="${value}"${nonceAttr(nonce, "script")}>`);
			else if (type === "preload") buffer.write(`<link${value.attrHtml}>`);
			else if (type === "inline-style") buffer.write(renderInlineStyle(value, nonce));
			else if (type === "head-tag") buffer.write(value);
		},
		shell(shellHtml, meta) {
			buffer.write(assembleDocument(shellHtml, meta.preloads, meta.preloadLinks, meta.inlineStyles, meta.tasks.length ? meta.tasks : "", nonce, meta.head, onHead));
		},
		...options.sink
	};
	const serializer = (options.serializer || createHydrationSerializer)({
		scopeId: options.renderId,
		plugins: options.plugins,
		onData: (payload) => sink.data(payload),
		onDone,
		onError: options.onError
	});
	let rootAssetsSerialized = false;
	const serializeRootAssets = () => {
		if (rootAssetsSerialized) return;
		rootAssetsSerialized = true;
		serializeFragmentAssets("", tracking.boundaryModules, context, renderId);
	};
	let holds = 0;
	const flushEnd = () => {
		if (!registry.size && !holds) {
			serializeRootAssets();
			queue(() => queue(() => {
				if (context.live.end) {
					const end = context.live.end;
					context.live.end = null;
					end();
				}
				flushStubBatch();
				serializer.flush();
			}));
		}
	};
	const registry = /* @__PURE__ */ new Map();
	const pendingSerialized = /* @__PURE__ */ new Map();
	const trackSerialized = (id, p) => {
		let settle;
		const raced = Promise.race([p, new Promise((r) => settle = r)]);
		pendingSerialized.set(id, settle);
		const drop = () => pendingSerialized.delete(id);
		p.then(drop, drop);
		return raced;
	};
	const abandonSubtree = (key) => {
		for (const [k, entry] of registry) if (k.length > key.length && k.startsWith(key)) {
			registry.delete(k);
			entry.resolve();
		}
		for (const [id, settle] of pendingSerialized) if (id.startsWith(key)) {
			pendingSerialized.delete(id);
			settle();
		}
	};
	const writeTasks = () => {
		if (tasks.length && !completed && firstFlushed) {
			buffer.write(`<script${nonceAttr(nonce, "script")}>${tasks}<\/script>`);
			tasks = "";
		}
		timer = null;
	};
	let context;
	let writable;
	let tmp = "";
	let tasks = "";
	let firstFlushed = false;
	let completed = false;
	let shellCompleted = false;
	let scriptFlushed = false;
	let headStyles;
	const revealGroups = /* @__PURE__ */ new Map();
	let timer = null;
	const emitTask = (task) => {
		pushTask(`${task}${!scriptFlushed ? ";" + REPLACE_SCRIPT : ""}`);
		scriptFlushed = true;
	};
	function resolveRevealKeys(groupOrKeys, release, consume) {
		if (Array.isArray(groupOrKeys)) return groupOrKeys.slice();
		let group = revealGroups.get(groupOrKeys);
		if (!group) {
			if (!release) return;
			group = {
				order: [],
				keys: /* @__PURE__ */ new Set(),
				released: true
			};
			revealGroups.set(groupOrKeys, group);
		} else if (release) group.released = true;
		if (!group.order.length) return;
		const keys = group.order.slice();
		if (consume) revealGroups.delete(groupOrKeys);
		return keys;
	}
	let rootHoles = null;
	let nextHoleId = 0;
	let buffer = { write(payload) {
		tmp += payload;
	} };
	const tracking = createAssetTracking();
	const headRegistry = createHeadRegistry();
	let headScriptFlushed = false;
	const emitHeadOps = (key, ops) => {
		const payload = JSON.stringify(ops).replace(/</g, "\\u003C");
		emitTask(`${!headScriptFlushed ? HEAD_SCRIPT : ""}(_$HY.hp=_$HY.hp||{})[${JSON.stringify(key)}]=${payload}`);
		headScriptFlushed = true;
	};
	sharedConfig.context = context = {
		async: true,
		nonce: options.nonce,
		live: {},
		registerHeadTags(tags) {
			registerHeadTags(headRegistry, context, tracking, (markup, gateEntry) => {
				if (!firstFlushed) {
					headRegistry.eagerHtml += markup;
					if (gateEntry) gateEntry.emitted = true;
				} else if (!gateEntry || !tracking.currentBoundaryId) sink.asset("head-tag", markup);
			}, nonce, tags);
		},
		registerAsset(type, value) {
			if (type === "preload") {
				const entry = registerPreloadLink(tracking, headRegistry, value, nonce);
				if (entry && firstFlushed) sink.asset("preload", entry);
				return;
			}
			if (type === "inline-style") {
				const entry = tracking.registerInlineStyle(value);
				if (firstFlushed && !tracking.currentBoundaryId && !entry.emitted) {
					entry.emitted = true;
					sink.asset("inline-style", entry);
				}
				return;
			}
			if (tracking.currentBoundaryId && type === "style") {
				let styles = tracking.boundaryStyles.get(tracking.currentBoundaryId);
				if (!styles) {
					styles = /* @__PURE__ */ new Set();
					tracking.boundaryStyles.set(tracking.currentBoundaryId, styles);
				}
				styles.add(value);
			}
			if (!tracking.emittedAssets.has(value)) {
				tracking.emittedAssets.add(value);
				if (firstFlushed) sink.asset(type, value);
			}
		},
		block(p) {
			if (!firstFlushed) blockingPromises.add(p);
		},
		hold() {
			holds++;
			let released = false;
			return () => {
				if (released) return;
				released = true;
				holds--;
				if (!holds) queue(flushEnd);
			};
		},
		replace(id, payloadFn) {
			if (firstFlushed) return;
			const placeholder = `<!--!$${id}-->`;
			const first = html.indexOf(placeholder);
			if (first === -1) return;
			const last = html.indexOf(`<!--!$/${id}-->`, first + placeholder.length);
			html = html.slice(0, first) + resolveSSRSync(escape(payloadFn())) + html.slice(last + placeholder.length + 1);
		},
		serialize(id, p, deferStream) {
			if (sharedConfig.context.noHydrate) return;
			if (p && typeof p === "object" && typeof p.then === "function") {
				if (!firstFlushed && deferStream) {
					blockingPromises.add(p);
					p.then((d) => serializer.write(id, d)).catch((e) => serializer.write(id, e));
					return;
				}
				p = trackSerialized(id, p);
				if (!firstFlushed && canBatchStubs && !shellCompleted) {
					(stubBatch ||= /* @__PURE__ */ new Map()).set(id, p);
					return;
				}
			}
			serializer.write(id, p);
		},
		escape,
		resolve: resolveSSRNode,
		ssr,
		registerFragment(key, options) {
			const revealGroup = options && options.revealGroup;
			if (revealGroup) {
				let group = revealGroups.get(revealGroup);
				if (!group) {
					group = {
						order: [],
						keys: /* @__PURE__ */ new Set(),
						released: false
					};
					revealGroups.set(revealGroup, group);
				}
				if (!group.keys.has(key)) {
					group.keys.add(key);
					group.order.push(key);
				}
				if (group.released) throw new Error("registerFragment() for reveal group '" + revealGroup + "' was called after revealFragments(). Ensure template payload is emitted before grouped reveal.");
			}
			if (!registry.has(key)) {
				let resolve, reject;
				const p = new Promise((r, rej) => (resolve = r, reject = rej));
				registry.set(key, { resolve: (err) => queue(() => queue(() => {
					err ? reject(err) : resolve(true);
					queue(flushEnd);
				})) });
				if (canBatchStubs && !shellCompleted) (stubBatch ||= /* @__PURE__ */ new Map()).set(key + "_fr", p);
				else serializer.write(key + "_fr", p);
			}
			return (value, error) => {
				if (registry.has(key)) {
					const item = registry.get(key);
					registry.delete(key);
					if (error) abandonSubtree(key);
					if (item.children) for (const k in item.children) value = replacePlaceholder(value, k, item.children[k]);
					const parentKey = waitForFragments(registry, key);
					if (parentKey) {
						const parent = registry.get(parentKey);
						parent.children ||= {};
						parent.children[key] = value !== void 0 ? value : "";
						serializeFragmentAssets(key, tracking.boundaryModules, context);
						propagateBoundaryStyles(key, parentKey, tracking);
						adoptHeadBoundary(headRegistry, key, parentKey);
						item.resolve();
						return;
					}
					if (!completed) {
						if (error) dropHeadBoundary(headRegistry, key);
						if (!firstFlushed) {
							queue(() => html = replacePlaceholder(html, key, value !== void 0 ? value : ""));
							serializeFragmentAssets(key, tracking.boundaryModules, context);
							item.resolve(error);
						} else {
							serializeFragmentAssets(key, tracking.boundaryModules, context);
							const styles = collectStreamStyles(key, tracking, headStyles);
							const headOps = error ? null : flushHeadFragment(headRegistry, key, nonce);
							if (headOps) emitHeadOps(key, headOps);
							sink.fragment(key, resolveSSRSelectValues(value !== void 0 ? value : " "), {
								styles,
								revealGroup,
								error
							});
							item.resolve(error);
						}
					}
				}
				return firstFlushed;
			};
		},
		revealFragments(groupOrKeys) {
			const keys = resolveRevealKeys(groupOrKeys, true, true);
			if (!keys) return;
			sink.reveal(keys, { fallback: false });
		},
		revealFallbacks(groupOrKeys) {
			const keys = resolveRevealKeys(groupOrKeys, false, false);
			if (!keys) return;
			sink.reveal(keys, { fallback: true });
		}
	};
	applyAssetTracking(context, tracking, manifest, noScripts);
	context.failRender = failRender;
	registerEntryAssets(manifest);
	let html = createRoot((d) => {
		dispose = d;
		const res = resolveSSRNode(escape(code()));
		if (!res.h.length) return res.t[0];
		rootHoles = [];
		let out = res.t[0];
		for (let i = 0; i < res.h.length; i++) {
			const id = nextHoleId++;
			rootHoles.push({
				id,
				fn: res.h[i]
			});
			out += `<!--rh${id}-->` + res.t[i + 1];
		}
		for (const p of res.p) blockingPromises.add(p);
		return out;
	}, { id: renderId });
	function resolveRootHoles() {
		if (!rootHoles) return true;
		const pending = [];
		for (const { id, fn } of rootHoles) {
			const marker = `<!--rh${id}-->`;
			const res = resolveSSRNode(fn);
			if (!res.h.length) html = html.replace(marker, res.t[0]);
			else {
				let out = res.t[0];
				for (let j = 0; j < res.h.length; j++) {
					const newId = nextHoleId++;
					pending.push({
						id: newId,
						fn: res.h[j]
					});
					out += `<!--rh${newId}-->` + res.t[j + 1];
				}
				html = html.replace(marker, out);
				for (const p of res.p) blockingPromises.add(p);
			}
		}
		if (pending.length) {
			rootHoles = pending;
			return false;
		}
		rootHoles = null;
		return true;
	}
	function doShell() {
		if (shellCompleted) return;
		sharedConfig.context = context;
		if (blockingPromises.size !== lastBlockingSize) return;
		if (!resolveRootHoles()) return;
		if (blockingPromises.size !== lastBlockingSize) return;
		if (!headShellReady(headRegistry, (p) => blockingPromises.add(p))) return;
		headStyles = /* @__PURE__ */ new Set();
		for (const url of tracking.emittedAssets) if (isCssUrl(url)) headStyles.add(url);
		serializeRootAssets();
		flushStubBatch();
		const head = renderShellHead(headRegistry, nonce, (k) => registry.has(k), noScripts);
		sink.shell(resolveSSRSelectValues(html), {
			preloads: tracking.emittedAssets,
			preloadLinks: tracking.preloadLinks,
			inlineStyles: tracking.inlineStyles,
			tasks,
			head
		});
		tasks = "";
		onCompleteShell && onCompleteShell({ write(v) {
			!completed && buffer.write(v);
		} });
		shellCompleted = true;
	}
	const MIN_DRAIN_TURNS = 8;
	let lastBlockingSize = -1;
	let lastRegistrySize = -1;
	let drainTurn = 0;
	const scheduleFlush = (fn, awaited) => {
		const attempt = () => {
			flushStubBatch();
			if (registry.size !== lastRegistrySize || drainTurn++ < MIN_DRAIN_TURNS) {
				if (registry.size !== lastRegistrySize) drainTurn = 0;
				lastRegistrySize = registry.size;
				queue(attempt);
				return;
			}
			fn();
		};
		const progressed = awaited !== lastBlockingSize;
		lastBlockingSize = awaited;
		lastRegistrySize = -1;
		drainTurn = 0;
		progressed ? queue(attempt) : setTimeout(attempt);
	};
	let cachedReadable;
	let consumer;
	const claimConsumer = (name) => {
		if (consumer && consumer !== name) throw new Error(`renderToStream result was already consumed via \`${consumer}\`; cannot also consume it via \`${name}\`. Use exactly one of \`pipe\`, \`pipeTo\`, or \`readable\`.`);
		consumer = name;
	};
	const pipeToImpl = (w) => {
		let resolve;
		const p = new Promise((r) => resolve = r);
		function flush() {
			allSettled(blockingPromises).then((awaited) => {
				scheduleFlush(() => {
					if (dead) return resolve();
					try {
						doShell();
					} catch (err) {
						failRender(err);
						return resolve();
					}
					if (!shellCompleted) return flush();
					const encoder = new TextEncoder();
					const writer = w.getWriter();
					let pendingWrites = Promise.resolve();
					let ended = false;
					const failed = () => {
						if (!ended) {
							abandon();
							resolve();
						}
					};
					writer.closed && writer.closed.catch(failed);
					buffer = writable = coalesceWrites((payload) => {
						pendingWrites = pendingWrites.then(() => writer.write(encoder.encode(payload))).catch(failed);
					}, () => {
						pendingWrites.then(() => {
							ended = true;
							writer.releaseLock();
							w.close().catch(() => {});
							resolve();
						});
					});
					buffer.write(tmp);
					buffer.flush();
					firstFlushed = true;
					if (completed) {
						dispose();
						writable.end();
					} else flushEnd();
				}, awaited);
			});
		}
		flush();
		return p;
	};
	return {
		then(onFulfilled, onRejected) {
			const freezeHead = () => {
				if (requestEvent && requestEvent.response) commitResponseStub(requestEvent.response);
			};
			return new Promise((resolve) => {
				function complete() {
					freezeHead();
					dispose();
					resolve(tmp);
				}
				if (onCompleteAll) {
					let ogComplete = onCompleteAll;
					onCompleteAll = (options) => {
						ogComplete(options);
						complete();
					};
				} else onCompleteAll = complete;
				function flush() {
					allSettled(blockingPromises).then((awaited) => {
						scheduleFlush(() => {
							try {
								if (blockingPromises.size !== lastBlockingSize || !resolveRootHoles() || blockingPromises.size !== lastBlockingSize || !headShellReady(headRegistry, (p) => blockingPromises.add(p))) return flush();
							} catch (err) {
								failRender(err);
								return resolve(tmp);
							}
							queue(flushEnd);
						}, awaited);
					});
				}
				flush();
			}).then(onFulfilled, onRejected);
		},
		pipe(w) {
			claimConsumer("pipe");
			function flush() {
				allSettled(blockingPromises).then((awaited) => {
					scheduleFlush(() => {
						if (dead) return;
						try {
							doShell();
						} catch (err) {
							failRender(err);
							try {
								w.end();
							} catch (_) {}
							return;
						}
						if (!shellCompleted) return flush();
						const sink = guardSink(w);
						buffer = writable = coalesceWrites(sink.write, sink.end);
						buffer.write(tmp);
						buffer.flush();
						firstFlushed = true;
						if (completed) {
							dispose();
							writable.end();
						} else flushEnd();
					}, awaited);
				});
			}
			flush();
		},
		pipeTo(w) {
			claimConsumer("pipeTo");
			return pipeToImpl(w);
		},
		get readable() {
			claimConsumer("readable");
			if (!cachedReadable) {
				const t = new TransformStream();
				pipeToImpl(t.writable);
				cachedReadable = t.readable;
			}
			return cachedReadable;
		}
	};
}
function HydrationScript(props) {
	return ssr(generateHydrationScript({
		nonce: scriptNonce(sharedConfig.context && sharedConfig.context.nonce),
		...props
	}));
}
function buildAsyncWrap(err, node) {
	const p = ssrHandleError(err);
	if (!p) return null;
	if (node.$rw) return {
		fn: node,
		p
	};
	const owner = getOwner();
	const live = sharedConfig.context && sharedConfig.context.liveHoles;
	const suppress = node.$lhSuppress || live && (live.suppressed || live.sweeping);
	if (!owner) {
		if (suppress) node.$lhSuppress = true;
		return {
			fn: node,
			p
		};
	}
	const fn = () => runWithOwner(owner, node);
	fn.$rw = true;
	if (suppress) fn.$lhSuppress = true;
	if (node.$lhSkip) fn.$lhSkip = true;
	if (node.$lhBinding) fn.$lhBinding = node.$lhBinding;
	return {
		fn,
		p
	};
}
function ssrFirstGroupHit(hole) {
	try {
		return hole();
	} catch (err) {
		return buildAsyncWrap(err, hole);
	}
}
function tryResolveFunctionHole(hole) {
	let value;
	try {
		value = hole();
	} catch (err) {
		return buildAsyncWrap(err, hole) || "";
	}
	const t = typeof value;
	if (t === "string") return value;
	if (t === "number") return "" + value;
	if (value == null || t === "boolean") return "";
	return tryResolveString(value);
}
function mergeTemplateInto(result, node) {
	result.t[result.t.length - 1] += node.t[0];
	if (node.t.length > 1) {
		result.t.push(...node.t.slice(1));
		result.h.push(...node.h);
		result.p.push(...node.p);
	}
}
function appendResolvedNode(result, node) {
	if (node.fn !== void 0) {
		result.h.push(node.fn);
		result.p.push(node.p);
		result.t.push("");
	} else if (node.merge !== void 0) mergeTemplateInto(result, node.merge);
	else resolveSSRNode(node.bail, result);
}
var _lastGroupFn = null;
var _lastGroupArr = null;
var _lastGroupErr = null;
function ssrGroupSlot(fn, idx) {
	return () => {
		if (idx > 0 && _lastGroupFn === fn) {
			if (_lastGroupArr !== null) return _lastGroupArr[idx];
			throw _lastGroupErr;
		}
		_lastGroupFn = fn;
		_lastGroupArr = null;
		_lastGroupErr = null;
		try {
			_lastGroupArr = fn();
			return _lastGroupArr[idx];
		} catch (err) {
			_lastGroupErr = err;
			throw err;
		}
	};
}
var holePositionCache = /* @__PURE__ */ new WeakMap();
function holeContentPositions(t) {
	let cached = holePositionCache.get(t);
	if (cached) return cached;
	const pos = [];
	const openOff = [];
	const closeOff = [];
	let inTag = false;
	let quote = "";
	let curOpen = null;
	let scanningName = false;
	for (let i = 0; i < t.length; i++) {
		const seg = t[i];
		let close = -1;
		const openAtStart = inTag;
		let reopened = false;
		for (let j = 0; j < seg.length; j++) {
			const ch = seg[j];
			if (quote) {
				if (ch === quote) quote = "";
			} else if (inTag) {
				if (scanningName && (ch === " " || ch === "	" || ch === "\n" || ch === ">" || ch === "/")) {
					scanningName = false;
					curOpen = {
						seg: i,
						off: j
					};
				}
				if (ch === "\"" || ch === "'") quote = ch;
				else if (ch === ">") {
					inTag = false;
					if (openAtStart && !reopened && close === -1) close = j;
					curOpen = null;
				}
			} else if (ch === "<") {
				inTag = true;
				scanningName = true;
				reopened = true;
				curOpen = null;
			}
		}
		if (inTag && scanningName) {
			scanningName = false;
			curOpen = {
				seg: i,
				off: seg.length
			};
		}
		closeOff.push(close);
		openOff.push(inTag && curOpen && curOpen.seg === i ? curOpen.off : -1);
		if (i < t.length - 1) pos.push(!inTag);
	}
	cached = {
		pos,
		openOff,
		closeOff
	};
	holePositionCache.set(t, cached);
	return cached;
}
function ssr(t) {
	const len = arguments.length;
	if (len === 1) return { t };
	let s = t[0];
	let result = null;
	let lastGroup = null;
	let lastGroupVal = null;
	let lastGroupIdx = 0;
	const live = sharedConfig.context && sharedConfig.context.liveHoles;
	const hp = live ? holeContentPositions(t) : null;
	let lastOpen = null;
	let cap = null;
	if (live && hp.openOff[0] >= 0) lastOpen = {
		r: null,
		seg: -1,
		off: hp.openOff[0]
	};
	const captureStart = () => {
		if (cap) return true;
		if (!lastOpen || live.suppressed || live.sweeping || !live.active()) return false;
		if (lastOpen.r !== result || result !== null && lastOpen.seg !== result.t.length - 1) return false;
		const id = live.mint();
		const inject = ` data-lha="${id}"`;
		let prefix;
		if (result === null) {
			s = s.slice(0, lastOpen.off) + inject + s.slice(lastOpen.off);
			prefix = s.slice(lastOpen.off + inject.length);
		} else {
			const seg = result.t[lastOpen.seg];
			result.t[lastOpen.seg] = seg.slice(0, lastOpen.off) + inject + seg.slice(lastOpen.off);
			prefix = result.t[lastOpen.seg].slice(lastOpen.off + inject.length);
		}
		cap = {
			id,
			parts: [prefix],
			base: prefix
		};
		return true;
	};
	for (let i = 1; i < len; i++) {
		const hole = arguments[i];
		const ht = typeof hole;
		if (ht === "string") {
			if (result === null) s += hole;
			else result.t[result.t.length - 1] += hole;
			if (cap) {
				cap.parts.push(hole);
				cap.base += hole;
			}
		} else if (ht === "number") {
			if (result === null) s += hole;
			else result.t[result.t.length - 1] += hole;
			if (cap) {
				cap.parts.push("" + hole);
				cap.base += hole;
			}
		} else if (hole == null || ht === "boolean");
		else if (ht === "function" && hole.$g) {
			let value;
			let hasValue = false;
			if (lastGroup !== hole) {
				const r = ssrFirstGroupHit(hole);
				if (r !== null) {
					lastGroup = hole;
					lastGroupVal = r;
					lastGroupIdx = 0;
					if (!Array.isArray(r) && result === null) {
						result = {
							t: [s],
							h: [],
							p: []
						};
						s = "";
					}
				}
			}
			if (lastGroup === hole) {
				if (Array.isArray(lastGroupVal)) {
					value = lastGroupVal[lastGroupIdx++];
					hasValue = true;
					if (live && !hp.pos[i - 1] && captureStart()) cap.parts.push({
						g: hole,
						i: lastGroupIdx - 1
					});
				} else {
					cap = null;
					result.h.push(ssrGroupSlot(lastGroupVal.fn, lastGroupIdx++));
					result.p.push(lastGroupVal.p);
					result.t.push("");
				}
			}
			if (hasValue) {
				const vt = typeof value;
				if (vt === "string" || vt === "number") {
					if (result === null) s += value;
					else result.t[result.t.length - 1] += value;
					if (cap && !hp.pos[i - 1]) cap.base += value;
				} else if (value == null || vt === "boolean");
				else if (result !== null) resolveSSRNode(value, result);
				else {
					const rs = tryResolveString(value);
					if (typeof rs === "string") s += rs;
					else {
						result = {
							t: [s],
							h: [],
							p: []
						};
						s = "";
						if (rs.merge !== void 0) mergeTemplateInto(result, rs.merge);
						else resolveSSRNode(rs.bail, result);
					}
				}
			}
		} else if (ht === "function") {
			let liveNode = null;
			if (live && hp.pos[i - 1] && (liveNode = live.content(hole)) !== null) {
				if (typeof liveNode === "string") {
					if (result === null) s += liveNode;
					else result.t[result.t.length - 1] += liveNode;
				} else {
					if (result === null) {
						result = {
							t: [s],
							h: [],
							p: []
						};
						s = "";
					}
					resolveSSRNode(liveNode, result);
				}
			} else if (result !== null) {
				const capturing = live && !hp.pos[i - 1] && captureStart();
				const li = capturing ? result.t.length - 1 : 0;
				const before = capturing ? result.t[li].length : 0;
				if (live) live.suppressed++;
				try {
					resolveSSRNode(hole, result);
				} finally {
					if (live) live.suppressed--;
				}
				if (capturing) {
					if (result.t.length - 1 === li) {
						cap.base += result.t[li].slice(before);
						cap.parts.push({ f: hole });
					} else cap = null;
				}
			} else {
				const capturing = live && !hp.pos[i - 1] && captureStart();
				const r = tryResolveFunctionHole(hole);
				if (typeof r === "string") {
					s += r;
					if (capturing) {
						cap.base += r;
						cap.parts.push({ f: hole });
					}
				} else {
					if (capturing) cap = null;
					result = {
						t: [s],
						h: [],
						p: []
					};
					s = "";
					appendResolvedNode(result, r);
				}
			}
		} else if (result !== null) resolveSSRNode(hole, result);
		else {
			const r = tryResolveString(hole);
			if (typeof r === "string") s += r;
			else {
				result = {
					t: [s],
					h: [],
					p: []
				};
				s = "";
				appendResolvedNode(result, r);
			}
		}
		const next = t[i];
		if (live) {
			if (cap) {
				const co = hp.closeOff[i];
				if (co >= 0) {
					const tail = next.slice(0, co);
					cap.parts.push(tail);
					cap.base += tail;
					live.attr(cap);
					cap = null;
				} else {
					cap.parts.push(next);
					cap.base += next;
				}
			}
			if (hp.openOff[i] >= 0) lastOpen = result === null ? {
				r: null,
				seg: -1,
				off: s.length + hp.openOff[i]
			} : {
				r: result,
				seg: result.t.length - 1,
				off: result.t[result.t.length - 1].length + hp.openOff[i]
			};
		}
		if (result === null) s += next;
		else result.t[result.t.length - 1] += next;
	}
	if (result === null) return { t: s };
	return result;
}
function ssrClassName(value) {
	if (typeof value === "number") return "" + value;
	if (!value) return "";
	if (typeof value === "string") return escape(value, true);
	value = classListToObject(value);
	let classKeys = Object.keys(value), result = "";
	for (let i = 0, len = classKeys.length; i < len; i++) {
		const key = classKeys[i], classValue = !!value[key];
		if (!key || key === "undefined" || !classValue) continue;
		i && (result += " ");
		result += escape(key, true);
	}
	return result;
}
function ssrStyle(value) {
	if (!value) return "";
	if (typeof value === "string") return escape(value, true);
	let result = "";
	const k = Object.keys(value);
	for (let i = 0; i < k.length; i++) {
		const s = escape(k[i], true);
		const v = value[k[i]];
		if (v != void 0) {
			if (i) result += ";";
			const r = escape(v, true);
			if (r != void 0 && r !== "undefined") result += `${s}:${r}`;
		}
	}
	return result;
}
function ssrElement(tag, props, children, needsId) {
	const hk = needsId ? ssrHydrationKey() : "";
	if (typeof props === "function") props = props();
	if (props == null) props = {};
	const skipChildren = VOID_ELEMENTS.test(tag);
	const keys = Object.keys(props);
	let result = `<${tag}${hk} `;
	for (let i = 0; i < keys.length; i++) {
		const prop = keys[i];
		if (tag === "textarea" && (prop === "value" || prop === "defaultValue")) {
			const value = props[prop];
			if (value !== null) children = escape(value);
			continue;
		}
		if (ChildProperties.has(prop)) {
			if (children === void 0 && !skipChildren) children = tag === "script" || tag === "style" || prop === "innerHTML" ? props[prop] : escape(props[prop]);
			continue;
		}
		const value = props[prop];
		if (prop === "style") result += `style="${ssrStyle(value)}"`;
		else if (prop === "class") result += `class="${ssrClassName(value)}"`;
		else if (value == void 0 || prop === "ref" || prop.slice(0, 2) === "on" || prop.slice(0, 5) === "prop:") continue;
		else if (typeof value === "boolean") {
			if (!value) continue;
			result += escape(prop);
		} else result += value === "" ? escape(prop) : `${escape(prop)}="${escape(value, true)}"`;
		if (i !== keys.length - 1) result += " ";
	}
	if (skipChildren) return { t: result + "/>" };
	if (typeof children === "function") children = children();
	return ssr([result + ">", `</${tag}>`], resolveSSRNode(children, void 0, true));
}
function ssrHydrationKey() {
	const hk = getHydrationKey();
	return hk ? ` _hk=${hk}` : "";
}
function decodeSSREntities(s) {
	return s.indexOf("&") < 0 ? s : s.replace(/&quot;/g, "\"").replace(/&lt;/g, "<").replace(/&amp;/g, "&");
}
var SELECT_VALUE_ATTR = /\svalue(?:="([^"]*)")?(?=[\s>]|$)/;
function tagEnd(html, i) {
	for (let j = i + 1; j < html.length; j++) {
		const c = html.charCodeAt(j);
		if (c === 34) {
			j = html.indexOf("\"", j + 1);
			if (j < 0) return -1;
		} else if (c === 62) return j;
	}
	return -1;
}
function optionText(html, from) {
	let t = "";
	let i = from;
	for (;;) {
		const lt = html.indexOf("<", i);
		if (lt < 0) return t + html.slice(i);
		t += html.slice(i, lt);
		if (!html.startsWith("<!--", lt)) return t;
		const ce = html.indexOf("-->", lt);
		if (ce < 0) return t;
		i = ce + 3;
	}
}
function tagIs(html, i, name) {
	if (!html.startsWith(name, i + 1)) return false;
	const c = html.charCodeAt(i + 1 + name.length);
	return c === 32 || c === 62 || c === 9 || c === 10 || c === 13;
}
var selectValuesActive = false;
function resolveSSRSelectValues(html) {
	if (!selectValuesActive) return html;
	let cand = html.indexOf("<select");
	if (cand < 0) return html;
	let out = "";
	let idx = 0;
	while (cand >= 0) {
		if (!tagIs(html, cand, "select")) {
			cand = html.indexOf("<select", cand + 7);
			continue;
		}
		const e0 = tagEnd(html, cand);
		if (e0 < 0) break;
		const open = html.slice(cand, e0 + 1);
		const m = SELECT_VALUE_ATTR.exec(open);
		if (!m) {
			cand = html.indexOf("<select", e0 + 1);
			continue;
		}
		const bound = decodeSSREntities(m[1] ?? "");
		const sel = {
			values: /\smultiple(?=[\s>=])/.test(open) ? bound.split(",") : [bound],
			strip: cand + m.index,
			stripEnd: cand + m.index + m[0].length,
			body: e0 + 1,
			marks: [],
			defaulted: false
		};
		let committed = false;
		let i = html.indexOf("<", e0 + 1);
		while (i >= 0) {
			let e;
			if (html.charCodeAt(i + 1) === 33) {
				e = html.charCodeAt(i + 2) === 45 ? html.indexOf("-->", i) : html.indexOf(">", i);
				if (e < 0) break;
				if (html.charCodeAt(i + 2) === 45) e += 2;
			} else {
				e = tagEnd(html, i);
				if (e < 0) break;
				if (html.startsWith("</select>", i)) {
					out += html.slice(idx, sel.strip) + html.slice(sel.stripEnd, sel.body);
					let seg = sel.body;
					if (!sel.defaulted) for (let k = 0; k < sel.marks.length; k++) {
						out += html.slice(seg, sel.marks[k]) + " selected";
						seg = sel.marks[k];
					}
					out += html.slice(seg, i);
					idx = i;
					committed = true;
					break;
				} else if (tagIs(html, i, "option")) {
					const attrs = html.slice(i + 7, e);
					if (/\sselected(?=[\s=]|$)/.test(attrs)) sel.defaulted = true;
					else {
						const vm = SELECT_VALUE_ATTR.exec(attrs);
						const value = vm ? decodeSSREntities(vm[1] ?? "") : decodeSSREntities(optionText(html, e + 1)).replace(/\s+/g, " ").trim();
						if (sel.values.includes(value)) sel.marks.push(e);
					}
				}
			}
			i = html.indexOf("<", e + 1);
		}
		if (!committed) break;
		cand = html.indexOf("<select", idx);
	}
	if (idx === 0) return html;
	return out + html.slice(idx);
}
function escape(s, attr) {
	const t = typeof s;
	if (t !== "string") {
		if (!attr && Array.isArray(s)) {
			const joined = tryJoinPlainSSRArray(s);
			if (joined !== void 0) return joined;
			const src = s;
			s = s.slice();
			for (let i = 0; i < s.length; i++) s[i] = escape(s[i]);
			if (src.$slot) s.$slot = true;
			return s;
		}
		if (!attr && t === "function") return escapeLate(s);
		if (attr) {
			if (s == null || t === "boolean" || t === "number") return s;
			return escape(String(s), attr);
		}
		return s;
	}
	const i = s.search(attr ? ESCAPE_ATTR : ESCAPE_CONTENT);
	if (i < 0) return s;
	return escapeSlow(s, attr, i);
}
function escapeLate(fn) {
	if (fn.$esc) return fn;
	const w = () => escape(fn());
	w.$esc = true;
	if (fn.$lhSkip) w.$lhSkip = true;
	if (fn.$lhSuppress) w.$lhSuppress = true;
	if (fn.$lhBinding) w.$lhBinding = fn.$lhBinding;
	return w;
}
var ESCAPE_CONTENT = /[&<]/;
var ESCAPE_ATTR = /[&"<]/;
function escapeSlow(s, attr, start) {
	if (attr) return escapeAttrSlow(s, start);
	const c0 = s.charCodeAt(start);
	let iDelim = c0 === 60 ? start : s.indexOf("<", start);
	let iAmp = c0 === 38 ? start : s.indexOf("&", start);
	let left = 0, out = "";
	while (iDelim >= 0 && iAmp >= 0) if (iDelim < iAmp) {
		if (left < iDelim) out += s.substring(left, iDelim);
		out += "&lt;";
		left = iDelim + 1;
		iDelim = s.indexOf("<", left);
	} else {
		if (left < iAmp) out += s.substring(left, iAmp);
		out += "&amp;";
		left = iAmp + 1;
		iAmp = s.indexOf("&", left);
	}
	if (iDelim >= 0) do {
		if (left < iDelim) out += s.substring(left, iDelim);
		out += "&lt;";
		left = iDelim + 1;
		iDelim = s.indexOf("<", left);
	} while (iDelim >= 0);
	else while (iAmp >= 0) {
		if (left < iAmp) out += s.substring(left, iAmp);
		out += "&amp;";
		left = iAmp + 1;
		iAmp = s.indexOf("&", left);
	}
	return left < s.length ? out + s.substring(left) : out;
}
function escapeAttrSlow(s, start) {
	const c0 = s.charCodeAt(start);
	let iQuot = c0 === 34 ? start : s.indexOf("\"", start);
	let iAmp = c0 === 38 ? start : s.indexOf("&", start);
	let iLt = c0 === 60 ? start : s.indexOf("<", start);
	let left = 0, out = "";
	for (;;) {
		let i = -1, ent;
		if (iQuot >= 0) {
			i = iQuot;
			ent = "&quot;";
		}
		if (iAmp >= 0 && (i < 0 || iAmp < i)) {
			i = iAmp;
			ent = "&amp;";
		}
		if (iLt >= 0 && (i < 0 || iLt < i)) {
			i = iLt;
			ent = "&lt;";
		}
		if (i < 0) break;
		if (left < i) out += s.substring(left, i);
		out += ent;
		left = i + 1;
		if (i === iQuot) iQuot = s.indexOf("\"", left);
		else if (i === iAmp) iAmp = s.indexOf("&", left);
		else iLt = s.indexOf("<", left);
	}
	return left < s.length ? out + s.substring(left) : out;
}
function tryJoinPlainSSRArray(nodes) {
	if (nodes.length === 0) return void 0;
	let out = "";
	for (let i = 0, len = nodes.length; i < len; i++) {
		const node = nodes[i];
		if (node == null || typeof node !== "object" || node.h || typeof node.t !== "string") return;
		out += node.t;
	}
	return out;
}
function getHydrationKey() {
	return sharedConfig.context && sharedConfig.getNextContextId();
}
function generateHydrationScript({ eventNames = ["click", "input"], nonce } = {}) {
	return `<script${nonce ? ` nonce="${escape(String(nonce), true)}"` : ""}>window._$HY||(e=>{let t=e=>e&&e.hasAttribute&&(e.hasAttribute("_hk")?e:t(e.host&&e.host.nodeType?e.host:e.parentNode));["${eventNames.join("\",\"")}"].forEach((o=>document.addEventListener(o,(o=>{if(!e.events)return;let s=t(o.composedPath&&o.composedPath()[0]||o.target);s&&!e.completed.has(s)&&e.events.push([s,o])}))))})(_$HY={events:[],completed:new WeakSet,r:{},fe(){}});<\/script><!--xs-->`;
}
function queue(fn) {
	return Promise.resolve().then(fn);
}
var deferFlush = typeof setImmediate === "function" ? setImmediate : (fn) => setTimeout(fn, 0);
function allSettled(promises) {
	let size = promises.size;
	return Promise.allSettled(promises).then(() => {
		if (promises.size !== size) return allSettled(promises);
		return size;
	});
}
function assembleDocument(html, emittedAssets, preloadLinks, inlineStyles, scripts, nonce, headTags, onHead) {
	const scriptTag = scripts ? `<script${nonceAttr(nonce, "script")}>${scripts}<\/script>` : "";
	const title = headTags ? headTags.title : null;
	let headTagsHtml = headTags ? headTags.html : "";
	const headPrelude = headTags ? headTags.prelude : "";
	if (!onHead && title == null && !headTagsHtml && !headPrelude && !(emittedAssets && emittedAssets.size) && !preloadLinks && !(inlineStyles && inlineStyles.size)) {
		if (!scriptTag) return html;
		const xs = html.indexOf("<!--xs-->");
		return xs === -1 ? html + scriptTag : html.slice(0, xs) + scriptTag + html.slice(xs);
	}
	if (headPrelude) {
		const open = html.match(/<head(?:\s[^>]*)?>/);
		if (open) {
			const at = open.index + open[0].length;
			html = html.slice(0, at) + headPrelude + html.slice(at);
		}
	}
	let headIdx = html.indexOf("</head>");
	if (headIdx === -1) {
		if (onHead) {
			let titleHtml = "";
			if (title != null) titleHtml = headTags.noScripts ? `<title data-dh="title">${escape(title)}</title>` : `<script${nonceAttr(nonce, "script")}>(function(x,t){(t=document.querySelector("title"))?(t.hasAttribute("data-dh")||t.setAttribute("data-dhf",t.textContent),t.textContent=x):(document.title=x,t=document.querySelector("title"));t&&t.setAttribute("data-dh","title")})(${JSON.stringify(title).replace(/</g, "\\u003C")})<\/script>`;
			onHead(headPrelude + headTagsHtml + titleHtml + renderHeadAssets(emittedAssets, preloadLinks, inlineStyles, nonce));
		}
		if (!scriptTag) return html;
		const xs = html.indexOf("<!--xs-->");
		return xs === -1 ? html + scriptTag : html.slice(0, xs) + scriptTag + html.slice(xs);
	}
	if (title != null) {
		const winner = escape(title);
		const open = html.match(/<head(?:\s[^>]*)?>/);
		const from = open ? open.index + open[0].length : 0;
		const m = /<title(\s[^>]*)?>([\s\S]*?)<\/title>/.exec(html.slice(from, headIdx));
		if (m) {
			const at = from + m.index;
			const stash = m[2].replace(/"/g, "&quot;");
			html = html.slice(0, at) + `<title${m[1] || ""} data-dh="title" data-dhf="${stash}">${winner}</title>` + html.slice(at + m[0].length);
			headIdx = html.indexOf("</head>");
		} else headTagsHtml = `<title data-dh="title">${winner}</title>` + headTagsHtml;
	}
	const head = headTagsHtml + renderHeadAssets(emittedAssets, preloadLinks, inlineStyles, nonce);
	if (!scriptTag) return html.slice(0, headIdx) + head + html.slice(headIdx);
	const xsIdx = html.indexOf("<!--xs-->");
	if (xsIdx === -1) return html.slice(0, headIdx) + head + html.slice(headIdx) + scriptTag;
	return xsIdx < headIdx ? html.slice(0, xsIdx) + scriptTag + html.slice(xsIdx, headIdx) + head + html.slice(headIdx) : html.slice(0, headIdx) + head + html.slice(headIdx, xsIdx) + scriptTag + html.slice(xsIdx);
}
function renderHeadAssets(emittedAssets, preloadLinks, inlineStyles, nonce) {
	let head = "";
	const styleAttr = nonceAttr(nonce, "style");
	const scriptAttr = nonceAttr(nonce, "script");
	if (emittedAssets && emittedAssets.size) for (const url of emittedAssets) head += isCssUrl(url) ? `<link rel="stylesheet" href="${url}"${styleAttr}>` : `<link rel="modulepreload" href="${url}"${scriptAttr}>`;
	if (preloadLinks) for (const entry of preloadLinks) head += `<link${entry.attrHtml}>`;
	if (inlineStyles && inlineStyles.size) for (const entry of inlineStyles.values()) {
		if (entry.emitted) continue;
		entry.emitted = true;
		head += renderInlineStyle(entry, nonce);
	}
	return head;
}
function serializeFragmentAssets(key, boundaryModules, context, name = key) {
	const map = boundaryModules.get(key);
	if (!map || !Object.keys(map).length) return;
	context.serialize(name + "_assets", { ...map });
}
function propagateBoundaryStyles(childKey, parentKey, tracking) {
	const childStyles = tracking.getBoundaryStyles(childKey);
	if (!childStyles) return;
	let parentStyles = tracking.boundaryStyles.get(parentKey);
	if (!parentStyles) {
		parentStyles = /* @__PURE__ */ new Set();
		tracking.boundaryStyles.set(parentKey, parentStyles);
	}
	for (const url of childStyles) parentStyles.add(url);
}
function collectStreamStyles(key, tracking, headStyles) {
	const styles = tracking.getBoundaryStyles(key);
	const links = [];
	const inline = [];
	if (!styles) return {
		links,
		inline
	};
	for (const entry of styles) if (typeof entry === "string") {
		if (!headStyles || !headStyles.has(entry)) links.push(entry);
	} else if (entry.emitted) continue;
	else if (entry.attrHtml !== void 0) {
		entry.emitted = true;
		links.push(entry);
	} else {
		entry.emitted = true;
		inline.push(entry);
	}
	return {
		links,
		inline
	};
}
function escapeStyleContent(content) {
	return content.replace(/<\/(style)/gi, "<\\/$1");
}
function renderInlineStyle(entry, nonce) {
	let attrs = "";
	let hasNonce = false;
	if (entry.attrs) for (const name in entry.attrs) {
		if (name.length === 5 && asciiLowerCase(name) === "nonce") hasNonce = true;
		attrs += ` ${name}="${escape(String(entry.attrs[name]), true)}"`;
	}
	return `<style${hasNonce ? "" : nonceAttr(nonce, "style")} data-asset="${escape(entry.id, true)}"${attrs}>${escapeStyleContent(entry.content)}</style>`;
}
function waitForFragments(registry, key) {
	for (const k of [...registry.keys()].reverse()) if (key.startsWith(k)) return k;
	return false;
}
function replacePlaceholder(html, key, value) {
	const marker = `<template id="pl-${key}">`;
	const close = `<!--pl-${key}-->`;
	const first = html.indexOf(marker);
	if (first === -1) return html;
	const last = html.indexOf(close, first + marker.length);
	return html.slice(0, first) + value + html.slice(last + close.length);
}
function classListToObject(classList) {
	if (Array.isArray(classList)) {
		const result = {};
		flattenClassList(classList, result);
		return result;
	}
	return classList;
}
function flattenClassList(list, result) {
	for (let i = 0, len = list.length; i < len; i++) {
		const item = list[i];
		if (Array.isArray(item)) flattenClassList(item, result);
		else if (typeof item === "object" && item != null) Object.assign(result, item);
		else if (typeof item !== "boolean" && (item || item === 0)) result[item] = true;
	}
}
function tryResolveString(node) {
	const t = typeof node;
	if (t === "string") return node;
	if (t === "number") return "" + node;
	if (node == null || t === "boolean") return "";
	if (t === "object") {
		if (Array.isArray(node)) {
			const joined = tryJoinPlainSSRArray(node);
			if (joined !== void 0) return joined;
			let s = "";
			let prevNonObj = false;
			for (let i = 0, len = node.length; i < len; i++) {
				const item = node[i];
				const itemNonObj = item !== null && typeof item !== "object";
				if (prevNonObj && itemNonObj) s += "<!--!$-->";
				prevNonObj = itemNonObj;
				const r = tryResolveString(item);
				if (typeof r !== "string") return { bail: node };
				s += r;
			}
			return s;
		}
		if (node.h && node.h.length > 0) return { merge: node };
		if (node.t === void 0) return "";
		return Array.isArray(node.t) ? node.t[0] : node.t;
	}
	if (t === "function") {
		let v;
		try {
			v = node();
		} catch (err) {
			return buildAsyncWrap(err, node) || "";
		}
		return tryResolveString(v);
	}
	return "";
}
function resolveSSRNode(node, result = {
	t: [""],
	h: [],
	p: []
}, top) {
	const t = typeof node;
	if (t === "string" || t === "number") result.t[result.t.length - 1] += node;
	else if (node == null || t === "boolean");
	else if (Array.isArray(node)) {
		const slotLive = node.$slot && sharedConfig.context && sharedConfig.context.liveHoles;
		if (slotLive) slotLive.suppressed++;
		try {
			let prevNonObj = false;
			for (let i = 0, len = node.length; i < len; i++) {
				const item = node[i];
				const itemNonObj = item !== null && typeof item !== "object";
				if (!top && prevNonObj && itemNonObj) result.t[result.t.length - 1] += `<!--!$-->`;
				prevNonObj = itemNonObj;
				resolveSSRNode(item, result);
			}
		} finally {
			if (slotLive) slotLive.suppressed--;
		}
	} else if (t === "object") {
		if (node.h) {
			result.t[result.t.length - 1] += node.t[0];
			if (node.t.length > 1) {
				result.t.push(...node.t.slice(1));
				result.h.push(...node.h);
				result.p.push(...node.p);
			}
		} else if (node.t !== void 0) result.t[result.t.length - 1] += node.t;
	} else if (t === "function") {
		const live = sharedConfig.context && sharedConfig.context.liveHoles;
		let liveNode = null;
		if (live && (liveNode = live.content(node)) !== null) {
			if (typeof liveNode === "string") result.t[result.t.length - 1] += liveNode;
			else resolveSSRNode(liveNode, result);
		} else try {
			resolveSSRNode(node(), result);
		} catch (err) {
			const wrap = buildAsyncWrap(err, node);
			if (wrap) {
				result.h.push(wrap.fn);
				result.p.push(wrap.p);
				result.t.push("");
			}
		}
	}
	return result;
}
function resolveSSRSync(node) {
	const res = resolveSSRNode(node);
	if (!res.h.length) return res.t[0];
	throw new Error("This value cannot be rendered synchronously. Are you missing a boundary?");
}
var RequestContext = Symbol.for("solid.RequestContext");
function getRequestEvent() {
	return globalThis[RequestContext] ? globalThis[RequestContext].getStore() || sharedConfig.context && sharedConfig.context.event || console.warn("RequestEvent is missing. This is most likely due to accessing `getRequestEvent` non-managed async scope in a partially polyfilled environment. Try moving it above all `await` calls.") : void 0;
}
function peekRequestEvent() {
	const store = globalThis[RequestContext];
	return store ? store.getStore() : void 0;
}
function createResponseStub() {
	return {
		status: void 0,
		statusText: void 0,
		headers: new Headers(),
		committed: false
	};
}
function createRequestEvent(request, init) {
	return {
		request,
		locals: {},
		response: createResponseStub(),
		...init
	};
}
function reportLostHeaderWrite(method, name) {
	const message = `Response header write dropped: headers.${method}(${JSON.stringify(String(name))}) ran after the response head was sent. Write headers before the shell flushes (or before the handler returns).`;
	console.error(message);
}
function commitResponseStub(stub, { allowLateLocation = false } = {}) {
	if (!stub || stub.committed) return stub;
	stub.committed = true;
	const headers = stub.headers;
	if (!headers || typeof headers.set !== "function") return stub;
	for (const method of [
		"set",
		"append",
		"delete"
	]) {
		const original = headers[method].bind(headers);
		headers[method] = function(name, ...rest) {
			if (allowLateLocation && method === "set" && String(name).toLowerCase() === "location") return original(name, ...rest);
			reportLostHeaderWrite(method, name);
		};
	}
	return stub;
}
var validRedirectStatuses = /*#__PURE__*/ new Set([
	301,
	302,
	303,
	307,
	308
]);
function getExpectedRedirectStatus(response) {
	if (response.status && validRedirectStatuses.has(response.status)) return response.status;
	return 302;
}
function mergeStubHeaders(target, stub) {
	if (!stub) return target;
	stub.headers.forEach((value, key) => {
		if (key !== "set-cookie") target.set(key, value);
	});
	const setCookies = stub.headers.getSetCookie ? stub.headers.getSetCookie() : [];
	for (const cookie of setCookies) target.append("set-cookie", cookie);
	return target;
}
function copyInitHeaders(init) {
	if (!init || !init.getSetCookie) return new Headers(init);
	const headers = new Headers();
	init.forEach((value, key) => {
		if (key !== "set-cookie") headers.append(key, value);
	});
	for (const cookie of init.getSetCookie()) headers.append("Set-Cookie", cookie);
	return headers;
}
var STUB_GAP_FILL_EXCLUDED = /*#__PURE__*/ new Set([
	ERROR_HEADER,
	BODY_FORMAT_HEADER,
	SINGLE_FLIGHT_HEADER,
	REVALIDATE_HEADER,
	REDIRECT_HEADER,
	"Location",
	...COMPOSED_BODY_FRAMING
].map((header) => header.toLowerCase()));
function fillsStubGap(key, headers, response) {
	if (key === "set-cookie" || STUB_GAP_FILL_EXCLUDED.has(key)) return false;
	if (response.body === null && (key === "content-type" || key === "content-length")) return false;
	return !headers.has(key);
}
function commitEventResponse(response, event = getRequestEvent()) {
	const stub = event && event.response;
	if (!stub || !stub.headers || stub.committed) return response;
	const cookies = stub.headers.getSetCookie ? stub.headers.getSetCookie() : [];
	commitResponseStub(stub);
	let hasGaps = false;
	stub.headers.forEach((value, key) => {
		if (fillsStubGap(key, response.headers, response)) hasGaps = true;
	});
	if (!cookies.length && !hasGaps) return response;
	const headers = copyInitHeaders(response.headers);
	for (const cookie of cookies) headers.append("Set-Cookie", cookie);
	stub.headers.forEach((value, key) => {
		if (fillsStubGap(key, headers, response)) headers.set(key, value);
	});
	return new Response(response.body, {
		status: response.status,
		statusText: response.statusText,
		headers
	});
}
function deriveHead(stub, responseInit = {}) {
	const headers = mergeStubHeaders(copyInitHeaders(responseInit.headers), stub);
	for (const header of COMPOSED_BODY_FRAMING) headers.delete(header);
	return {
		status: stub && stub.status || responseInit.status || 200,
		statusText: stub && stub.statusText || responseInit.statusText || void 0,
		headers
	};
}
function createSSRResponse(result, event, options = {}) {
	const stub = event && event.response;
	const { responseInit, transformChunk } = options;
	const nonce = normalizeNonce(options.nonce);
	if (typeof result === "string") {
		if (stub) commitResponseStub(stub);
		const head = deriveHead(stub, responseInit);
		if (stub && stub.headers.get("Location")) return new Response(null, {
			status: getExpectedRedirectStatus(stub),
			headers: head.headers
		});
		if (!head.headers.has("content-type")) head.headers.set("content-type", "text/html; charset=utf-8");
		return new Response(transformChunk ? transformChunk(result) : result, {
			status: head.status,
			statusText: head.statusText,
			headers: head.headers
		});
	}
	const encoder = new TextEncoder();
	return new Promise((resolve) => {
		let controller;
		let closed = false;
		let flushed = false;
		const enqueue = (value) => {
			if (closed || !controller) return;
			try {
				controller.enqueue(encoder.encode(value));
			} catch {
				closed = true;
			}
		};
		result.pipe({
			write(chunk) {
				if (!flushed) {
					flushed = true;
					if (stub) commitResponseStub(stub, { allowLateLocation: true });
					const head = deriveHead(stub, responseInit);
					if (stub && stub.headers.get("Location")) {
						closed = true;
						resolve(new Response(null, {
							status: getExpectedRedirectStatus(stub),
							headers: head.headers
						}));
						return;
					}
					if (!head.headers.has("content-type")) head.headers.set("content-type", "text/html; charset=utf-8");
					resolve(new Response(new ReadableStream({
						start(c) {
							controller = c;
						},
						cancel() {
							closed = true;
						}
					}), {
						status: head.status,
						statusText: head.statusText,
						headers: head.headers
					}));
				}
				enqueue(transformChunk ? transformChunk(chunk) : chunk);
			},
			end() {
				if (closed || !controller) return;
				const location = stub && stub.headers.get("Location");
				if (location && isHttpNavigationTarget(location)) {
					const attr = nonceAttr(nonce, "script");
					enqueue(`<script${attr}>window.location=${JSON.stringify(location).replace(/</g, "\\u003c")}<\/script>`);
				}
				closed = true;
				try {
					controller.close();
				} catch {}
			}
		});
	});
}
setContainerTraceResolver(getProjectionTrace);
var statusLedgers = /* @__PURE__ */ new WeakMap();
function httpStatus(code, text) {
	const event = getRequestEvent();
	const response = event && event.response;
	if (response && !response.committed) {
		let ledger = statusLedgers.get(response);
		if (!ledger) {
			ledger = {
				base: {
					status: response.status,
					statusText: response.statusText
				},
				live: []
			};
			statusLedgers.set(response, ledger);
		}
		const declaration = {
			status: code,
			statusText: text
		};
		ledger.live.push(declaration);
		response.status = code;
		response.statusText = text;
		onCleanup(() => {
			if (response.committed) return;
			const index = ledger.live.indexOf(declaration);
			if (index < 0) return;
			ledger.live.splice(index, 1);
			const effective = ledger.live.length ? ledger.live[ledger.live.length - 1] : ledger.base;
			response.status = effective.status;
			response.statusText = effective.statusText;
			if (!ledger.live.length) statusLedgers.delete(response);
		});
	}
}
function provideRequestEvent(init, cb) {
	return (globalThis[RequestContext] = globalThis[RequestContext] || new AsyncLocalStorage()).run(init, cb);
}
var _virtual_solid_manifest_default = {
	"virtual:solid-ssr-entry-client.tsx": {
		"file": "assets/virtual_solid-ssr-entry-client-vXr4AcT0.js",
		"name": "virtual_solid-ssr-entry-client",
		"src": "virtual:solid-ssr-entry-client.tsx",
		"isEntry": true,
		"css": ["assets/virtual_solid-ssr-entry-client-fvpRBgw-.css"]
	},
	"_virtual_solid-ssr-entry-client-fvpRBgw-.css": {
		"file": "assets/virtual_solid-ssr-entry-client-fvpRBgw-.css",
		"src": "_virtual_solid-ssr-entry-client-fvpRBgw-.css"
	},
	"_base": "/",
	"_entry": "virtual:solid-ssr-entry-client.tsx"
};
var _tmpl$$2 = [
	"<html",
	" lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><!--$-->",
	"<!--/--></head><body>",
	"</body></html>"
];
function Document(props) {
	return ssr(_tmpl$$2, ssrHydrationKey(), escape(HydrationScript({})), ssrScope(() => {
		return escape(props.children);
	}));
}
/** Gray light alpha scale. */
var grayA = {
	grayA1: "#00000003",
	grayA2: "#00000006",
	grayA3: "#0000000f",
	grayA4: "#00000017",
	grayA5: "#0000001f",
	grayA6: "#00000026",
	grayA7: "#00000031",
	grayA8: "#00000044",
	grayA9: "#00000072",
	grayA10: "#0000007c",
	grayA11: "#0000009b",
	grayA12: "#000000df"
};
/** Gray dark alpha scale. */
var grayDarkA = {
	grayA1: "#00000000",
	grayA2: "#ffffff09",
	grayA3: "#ffffff12",
	grayA4: "#ffffff1b",
	grayA5: "#ffffff22",
	grayA6: "#ffffff2c",
	grayA7: "#ffffff3b",
	grayA8: "#ffffff55",
	grayA9: "#ffffff64",
	grayA10: "#ffffff72",
	grayA11: "#ffffffaf",
	grayA12: "#ffffffed"
};
/** Mauve light alpha scale. */
var mauveA = {
	mauveA1: "#55005503",
	mauveA2: "#2b005506",
	mauveA3: "#30004010",
	mauveA4: "#20003618",
	mauveA5: "#20003820",
	mauveA6: "#14003527",
	mauveA7: "#10003332",
	mauveA8: "#08003145",
	mauveA9: "#05001d73",
	mauveA10: "#0500197d",
	mauveA11: "#0400119c",
	mauveA12: "#020008e0"
};
/** Mauve dark alpha scale. */
var mauveDarkA = {
	mauveA1: "#00000000",
	mauveA2: "#f5f4f609",
	mauveA3: "#ebeaf814",
	mauveA4: "#eee5f81d",
	mauveA5: "#efe6fe25",
	mauveA6: "#f1e6fd30",
	mauveA7: "#eee9ff40",
	mauveA8: "#eee7ff5d",
	mauveA9: "#eae6fd6e",
	mauveA10: "#ece9fd7c",
	mauveA11: "#f5f1ffb7",
	mauveA12: "#fdfdffef"
};
/** Slate light alpha scale. */
var slateA = {
	slateA1: "#00005503",
	slateA2: "#00005506",
	slateA3: "#0000330f",
	slateA4: "#00002d17",
	slateA5: "#0009321f",
	slateA6: "#00002f26",
	slateA7: "#00062e32",
	slateA8: "#00083046",
	slateA9: "#00051d74",
	slateA10: "#00071b7f",
	slateA11: "#0007149f",
	slateA12: "#000509e3"
};
/** Slate dark alpha scale. */
var slateDarkA = {
	slateA1: "#00000000",
	slateA2: "#d8f4f609",
	slateA3: "#ddeaf814",
	slateA4: "#d3edf81d",
	slateA5: "#d9edfe25",
	slateA6: "#d6ebfd30",
	slateA7: "#d9edff40",
	slateA8: "#d9edff5d",
	slateA9: "#dfebfd6d",
	slateA10: "#e5edfd7b",
	slateA11: "#f1f7feb5",
	slateA12: "#fcfdffef"
};
/** Sage light alpha scale. */
var sageA = {
	sageA1: "#00804004",
	sageA2: "#00402008",
	sageA3: "#002d1e11",
	sageA4: "#001f1519",
	sageA5: "#00180820",
	sageA6: "#00140d28",
	sageA7: "#00140a34",
	sageA8: "#000f0847",
	sageA9: "#00110b79",
	sageA10: "#00100a83",
	sageA11: "#000a07a0",
	sageA12: "#000805e5"
};
/** Sage dark alpha scale. */
var sageDarkA = {
	sageA1: "#00000000",
	sageA2: "#f0f2f108",
	sageA3: "#f3f5f412",
	sageA4: "#f2fefd1a",
	sageA5: "#f1fbfa22",
	sageA6: "#edfbf42d",
	sageA7: "#edfcf73c",
	sageA8: "#ebfdf657",
	sageA9: "#dffdf266",
	sageA10: "#e5fdf674",
	sageA11: "#f4fefbb0",
	sageA12: "#fdfffeed"
};
/** Olive light alpha scale. */
var oliveA = {
	oliveA1: "#00550003",
	oliveA2: "#00490007",
	oliveA3: "#00200010",
	oliveA4: "#00160018",
	oliveA5: "#00180020",
	oliveA6: "#00140028",
	oliveA7: "#000f0033",
	oliveA8: "#040f0047",
	oliveA9: "#050f0078",
	oliveA10: "#040e0082",
	oliveA11: "#020a00a0",
	oliveA12: "#010600e3"
};
/** Olive dark alpha scale. */
var oliveDarkA = {
	oliveA1: "#00000000",
	oliveA2: "#f1f2f008",
	oliveA3: "#f4f5f312",
	oliveA4: "#f3fef21a",
	oliveA5: "#f2fbf122",
	oliveA6: "#f4faed2c",
	oliveA7: "#f2fced3b",
	oliveA8: "#edfdeb57",
	oliveA9: "#ebfde766",
	oliveA10: "#f0fdec74",
	oliveA11: "#f6fef4b0",
	oliveA12: "#fdfffded"
};
/** Sand light alpha scale. */
var sandA = {
	sandA1: "#55550003",
	sandA2: "#25250007",
	sandA3: "#20100010",
	sandA4: "#1f150019",
	sandA5: "#1f180021",
	sandA6: "#19130029",
	sandA7: "#19140035",
	sandA8: "#1915014a",
	sandA9: "#0f0f0079",
	sandA10: "#0c0c0083",
	sandA11: "#080800a1",
	sandA12: "#060500e3"
};
/** Sand dark alpha scale. */
var sandDarkA = {
	sandA1: "#00000000",
	sandA2: "#f4f4f309",
	sandA3: "#f6f6f513",
	sandA4: "#fefef31b",
	sandA5: "#fbfbeb23",
	sandA6: "#fffaed2d",
	sandA7: "#fffbed3c",
	sandA8: "#fff9eb57",
	sandA9: "#fffae965",
	sandA10: "#fffdee73",
	sandA11: "#fffcf4b0",
	sandA12: "#fffffded"
};
/** Black light alpha scale. */
var blackA = {
	blackA1: "rgba(0, 0, 0, 0.05)",
	blackA2: "rgba(0, 0, 0, 0.1)",
	blackA3: "rgba(0, 0, 0, 0.15)",
	blackA4: "rgba(0, 0, 0, 0.2)",
	blackA5: "rgba(0, 0, 0, 0.3)",
	blackA6: "rgba(0, 0, 0, 0.4)",
	blackA7: "rgba(0, 0, 0, 0.5)",
	blackA8: "rgba(0, 0, 0, 0.6)",
	blackA9: "rgba(0, 0, 0, 0.7)",
	blackA10: "rgba(0, 0, 0, 0.8)",
	blackA11: "rgba(0, 0, 0, 0.9)",
	blackA12: "rgba(0, 0, 0, 0.95)"
};
/** Gray light scale. */
var gray = {
	gray1: "#fcfcfc",
	gray2: "#f9f9f9",
	gray3: "#f0f0f0",
	gray4: "#e8e8e8",
	gray5: "#e0e0e0",
	gray6: "#d9d9d9",
	gray7: "#cecece",
	gray8: "#bbbbbb",
	gray9: "#8d8d8d",
	gray10: "#838383",
	gray11: "#646464",
	gray12: "#202020"
};
/** Gray dark scale. */
var grayDark = {
	gray1: "#111111",
	gray2: "#191919",
	gray3: "#222222",
	gray4: "#2a2a2a",
	gray5: "#313131",
	gray6: "#3a3a3a",
	gray7: "#484848",
	gray8: "#606060",
	gray9: "#6e6e6e",
	gray10: "#7b7b7b",
	gray11: "#b4b4b4",
	gray12: "#eeeeee"
};
/** Mauve light scale. */
var mauve = {
	mauve1: "#fdfcfd",
	mauve2: "#faf9fb",
	mauve3: "#f2eff3",
	mauve4: "#eae7ec",
	mauve5: "#e3dfe6",
	mauve6: "#dbd8e0",
	mauve7: "#d0cdd7",
	mauve8: "#bcbac7",
	mauve9: "#8e8c99",
	mauve10: "#84828e",
	mauve11: "#65636d",
	mauve12: "#211f26"
};
/** Mauve dark scale. */
var mauveDark = {
	mauve1: "#121113",
	mauve2: "#1a191b",
	mauve3: "#232225",
	mauve4: "#2b292d",
	mauve5: "#323035",
	mauve6: "#3c393f",
	mauve7: "#49474e",
	mauve8: "#625f69",
	mauve9: "#6f6d78",
	mauve10: "#7c7a85",
	mauve11: "#b5b2bc",
	mauve12: "#eeeef0"
};
/** Slate light scale. */
var slate = {
	slate1: "#fcfcfd",
	slate2: "#f9f9fb",
	slate3: "#f0f0f3",
	slate4: "#e8e8ec",
	slate5: "#e0e1e6",
	slate6: "#d9d9e0",
	slate7: "#cdced6",
	slate8: "#b9bbc6",
	slate9: "#8b8d98",
	slate10: "#80838d",
	slate11: "#60646c",
	slate12: "#1c2024"
};
/** Slate dark scale. */
var slateDark = {
	slate1: "#111113",
	slate2: "#18191b",
	slate3: "#212225",
	slate4: "#272a2d",
	slate5: "#2e3135",
	slate6: "#363a3f",
	slate7: "#43484e",
	slate8: "#5a6169",
	slate9: "#696e77",
	slate10: "#777b84",
	slate11: "#b0b4ba",
	slate12: "#edeef0"
};
/** Sage light scale. */
var sage = {
	sage1: "#fbfdfc",
	sage2: "#f7f9f8",
	sage3: "#eef1f0",
	sage4: "#e6e9e8",
	sage5: "#dfe2e0",
	sage6: "#d7dad9",
	sage7: "#cbcfcd",
	sage8: "#b8bcba",
	sage9: "#868e8b",
	sage10: "#7c8481",
	sage11: "#5f6563",
	sage12: "#1a211e"
};
/** Sage dark scale. */
var sageDark = {
	sage1: "#101211",
	sage2: "#171918",
	sage3: "#202221",
	sage4: "#272a29",
	sage5: "#2e3130",
	sage6: "#373b39",
	sage7: "#444947",
	sage8: "#5b625f",
	sage9: "#63706b",
	sage10: "#717d79",
	sage11: "#adb5b2",
	sage12: "#eceeed"
};
/** Olive light scale. */
var olive = {
	olive1: "#fcfdfc",
	olive2: "#f8faf8",
	olive3: "#eff1ef",
	olive4: "#e7e9e7",
	olive5: "#dfe2df",
	olive6: "#d7dad7",
	olive7: "#cccfcc",
	olive8: "#b9bcb8",
	olive9: "#898e87",
	olive10: "#7f847d",
	olive11: "#60655f",
	olive12: "#1d211c"
};
/** Olive dark scale. */
var oliveDark = {
	olive1: "#111210",
	olive2: "#181917",
	olive3: "#212220",
	olive4: "#282a27",
	olive5: "#2f312e",
	olive6: "#383a36",
	olive7: "#454843",
	olive8: "#5c625b",
	olive9: "#687066",
	olive10: "#767d74",
	olive11: "#afb5ad",
	olive12: "#eceeec"
};
/** Sand light scale. */
var sand = {
	sand1: "#fdfdfc",
	sand2: "#f9f9f8",
	sand3: "#f1f0ef",
	sand4: "#e9e8e6",
	sand5: "#e2e1de",
	sand6: "#dad9d6",
	sand7: "#cfceca",
	sand8: "#bcbbb5",
	sand9: "#8d8d86",
	sand10: "#82827c",
	sand11: "#63635e",
	sand12: "#21201c"
};
/** Sand dark scale. */
var sandDark = {
	sand1: "#111110",
	sand2: "#191918",
	sand3: "#222221",
	sand4: "#2a2a28",
	sand5: "#31312e",
	sand6: "#3b3a37",
	sand7: "#494844",
	sand8: "#62605b",
	sand9: "#6f6d66",
	sand10: "#7c7b74",
	sand11: "#b5b3ad",
	sand12: "#eeeeec"
};
/** Tomato light scale. */
var tomato = {
	tomato1: "#fffcfc",
	tomato2: "#fff8f7",
	tomato3: "#feebe7",
	tomato4: "#ffdcd3",
	tomato5: "#ffcdc2",
	tomato6: "#fdbdaf",
	tomato7: "#f5a898",
	tomato8: "#ec8e7b",
	tomato9: "#e54d2e",
	tomato10: "#dd4425",
	tomato11: "#d13415",
	tomato12: "#5c271f"
};
/** Tomato dark scale. */
var tomatoDark = {
	tomato1: "#181111",
	tomato2: "#1f1513",
	tomato3: "#391714",
	tomato4: "#4e1511",
	tomato5: "#5e1c16",
	tomato6: "#6e2920",
	tomato7: "#853a2d",
	tomato8: "#ac4d39",
	tomato9: "#e54d2e",
	tomato10: "#ec6142",
	tomato11: "#ff977d",
	tomato12: "#fbd3cb"
};
/** Red light scale. */
var red = {
	red1: "#fffcfc",
	red2: "#fff7f7",
	red3: "#feebec",
	red4: "#ffdbdc",
	red5: "#ffcdce",
	red6: "#fdbdbe",
	red7: "#f4a9aa",
	red8: "#eb8e90",
	red9: "#e5484d",
	red10: "#dc3e42",
	red11: "#ce2c31",
	red12: "#641723"
};
/** Red dark scale. */
var redDark = {
	red1: "#191111",
	red2: "#201314",
	red3: "#3b1219",
	red4: "#500f1c",
	red5: "#611623",
	red6: "#72232d",
	red7: "#8c333a",
	red8: "#b54548",
	red9: "#e5484d",
	red10: "#ec5d5e",
	red11: "#ff9592",
	red12: "#ffd1d9"
};
/** Ruby light scale. */
var ruby = {
	ruby1: "#fffcfd",
	ruby2: "#fff7f8",
	ruby3: "#feeaed",
	ruby4: "#ffdce1",
	ruby5: "#ffced6",
	ruby6: "#f8bfc8",
	ruby7: "#efacb8",
	ruby8: "#e592a3",
	ruby9: "#e54666",
	ruby10: "#dc3b5d",
	ruby11: "#ca244d",
	ruby12: "#64172b"
};
/** Ruby dark scale. */
var rubyDark = {
	ruby1: "#191113",
	ruby2: "#1e1517",
	ruby3: "#3a141e",
	ruby4: "#4e1325",
	ruby5: "#5e1a2e",
	ruby6: "#6f2539",
	ruby7: "#883447",
	ruby8: "#b3445a",
	ruby9: "#e54666",
	ruby10: "#ec5a72",
	ruby11: "#ff949d",
	ruby12: "#fed2e1"
};
/** Crimson light scale. */
var crimson = {
	crimson1: "#fffcfd",
	crimson2: "#fef7f9",
	crimson3: "#ffe9f0",
	crimson4: "#fedce7",
	crimson5: "#facedd",
	crimson6: "#f3bed1",
	crimson7: "#eaacc3",
	crimson8: "#e093b2",
	crimson9: "#e93d82",
	crimson10: "#df3478",
	crimson11: "#cb1d63",
	crimson12: "#621639"
};
/** Crimson dark scale. */
var crimsonDark = {
	crimson1: "#191114",
	crimson2: "#201318",
	crimson3: "#381525",
	crimson4: "#4d122f",
	crimson5: "#5c1839",
	crimson6: "#6d2545",
	crimson7: "#873356",
	crimson8: "#b0436e",
	crimson9: "#e93d82",
	crimson10: "#ee518a",
	crimson11: "#ff92ad",
	crimson12: "#fdd3e8"
};
/** Pink light scale. */
var pink = {
	pink1: "#fffcfe",
	pink2: "#fef7fb",
	pink3: "#fee9f5",
	pink4: "#fbdcef",
	pink5: "#f6cee7",
	pink6: "#efbfdd",
	pink7: "#e7acd0",
	pink8: "#dd93c2",
	pink9: "#d6409f",
	pink10: "#cf3897",
	pink11: "#c2298a",
	pink12: "#651249"
};
/** Pink dark scale. */
var pinkDark = {
	pink1: "#191117",
	pink2: "#21121d",
	pink3: "#37172f",
	pink4: "#4b143d",
	pink5: "#591c47",
	pink6: "#692955",
	pink7: "#833869",
	pink8: "#a84885",
	pink9: "#d6409f",
	pink10: "#de51a8",
	pink11: "#ff8dcc",
	pink12: "#fdd1ea"
};
/** Plum light scale. */
var plum = {
	plum1: "#fefcff",
	plum2: "#fdf7fd",
	plum3: "#fbebfb",
	plum4: "#f7def8",
	plum5: "#f2d1f3",
	plum6: "#e9c2ec",
	plum7: "#deade3",
	plum8: "#cf91d8",
	plum9: "#ab4aba",
	plum10: "#a144af",
	plum11: "#953ea3",
	plum12: "#53195d"
};
/** Plum dark scale. */
var plumDark = {
	plum1: "#181118",
	plum2: "#201320",
	plum3: "#351a35",
	plum4: "#451d47",
	plum5: "#512454",
	plum6: "#5e3061",
	plum7: "#734079",
	plum8: "#92549c",
	plum9: "#ab4aba",
	plum10: "#b658c4",
	plum11: "#e796f3",
	plum12: "#f4d4f4"
};
/** Purple light scale. */
var purple = {
	purple1: "#fefcfe",
	purple2: "#fbf7fe",
	purple3: "#f7edfe",
	purple4: "#f2e2fc",
	purple5: "#ead5f9",
	purple6: "#e0c4f4",
	purple7: "#d1afec",
	purple8: "#be93e4",
	purple9: "#8e4ec6",
	purple10: "#8347b9",
	purple11: "#8145b5",
	purple12: "#402060"
};
/** Purple dark scale. */
var purpleDark = {
	purple1: "#18111b",
	purple2: "#1e1523",
	purple3: "#301c3b",
	purple4: "#3d224e",
	purple5: "#48295c",
	purple6: "#54346b",
	purple7: "#664282",
	purple8: "#8457aa",
	purple9: "#8e4ec6",
	purple10: "#9a5cd0",
	purple11: "#d19dff",
	purple12: "#ecd9fa"
};
/** Violet light scale. */
var violet = {
	violet1: "#fdfcfe",
	violet2: "#faf8ff",
	violet3: "#f4f0fe",
	violet4: "#ebe4ff",
	violet5: "#e1d9ff",
	violet6: "#d4cafe",
	violet7: "#c2b5f5",
	violet8: "#aa99ec",
	violet9: "#6e56cf",
	violet10: "#654dc4",
	violet11: "#6550b9",
	violet12: "#2f265f"
};
/** Violet dark scale. */
var violetDark = {
	violet1: "#14121f",
	violet2: "#1b1525",
	violet3: "#291f43",
	violet4: "#33255b",
	violet5: "#3c2e69",
	violet6: "#473876",
	violet7: "#56468b",
	violet8: "#6958ad",
	violet9: "#6e56cf",
	violet10: "#7d66d9",
	violet11: "#baa7ff",
	violet12: "#e2ddfe"
};
/** Iris light scale. */
var iris = {
	iris1: "#fdfdff",
	iris2: "#f8f8ff",
	iris3: "#f0f1fe",
	iris4: "#e6e7ff",
	iris5: "#dadcff",
	iris6: "#cbcdff",
	iris7: "#b8baf8",
	iris8: "#9b9ef0",
	iris9: "#5b5bd6",
	iris10: "#5151cd",
	iris11: "#5753c6",
	iris12: "#272962"
};
/** Iris dark scale. */
var irisDark = {
	iris1: "#13131e",
	iris2: "#171625",
	iris3: "#202248",
	iris4: "#262a65",
	iris5: "#303374",
	iris6: "#3d3e82",
	iris7: "#4a4a95",
	iris8: "#5958b1",
	iris9: "#5b5bd6",
	iris10: "#6e6ade",
	iris11: "#b1a9ff",
	iris12: "#e0dffe"
};
/** Indigo light scale. */
var indigo = {
	indigo1: "#fdfdfe",
	indigo2: "#f7f9ff",
	indigo3: "#edf2fe",
	indigo4: "#e1e9ff",
	indigo5: "#d2deff",
	indigo6: "#c1d0ff",
	indigo7: "#abbdf9",
	indigo8: "#8da4ef",
	indigo9: "#3e63dd",
	indigo10: "#3358d4",
	indigo11: "#3a5bc7",
	indigo12: "#1f2d5c"
};
/** Indigo dark scale. */
var indigoDark = {
	indigo1: "#11131f",
	indigo2: "#141726",
	indigo3: "#182449",
	indigo4: "#1d2e62",
	indigo5: "#253974",
	indigo6: "#304384",
	indigo7: "#3a4f97",
	indigo8: "#435db1",
	indigo9: "#3e63dd",
	indigo10: "#5472e4",
	indigo11: "#9eb1ff",
	indigo12: "#d6e1ff"
};
/** Blue light scale. */
var blue = {
	blue1: "#fbfdff",
	blue2: "#f4faff",
	blue3: "#e6f4fe",
	blue4: "#d5efff",
	blue5: "#c2e5ff",
	blue6: "#acd8fc",
	blue7: "#8ec8f6",
	blue8: "#5eb1ef",
	blue9: "#0090ff",
	blue10: "#0588f0",
	blue11: "#0d74ce",
	blue12: "#113264"
};
/** Blue dark scale. */
var blueDark = {
	blue1: "#0d1520",
	blue2: "#111927",
	blue3: "#0d2847",
	blue4: "#003362",
	blue5: "#004074",
	blue6: "#104d87",
	blue7: "#205d9e",
	blue8: "#2870bd",
	blue9: "#0090ff",
	blue10: "#3b9eff",
	blue11: "#70b8ff",
	blue12: "#c2e6ff"
};
/** Cyan light scale. */
var cyan = {
	cyan1: "#fafdfe",
	cyan2: "#f2fafb",
	cyan3: "#def7f9",
	cyan4: "#caf1f6",
	cyan5: "#b5e9f0",
	cyan6: "#9ddde7",
	cyan7: "#7dcedc",
	cyan8: "#3db9cf",
	cyan9: "#00a2c7",
	cyan10: "#0797b9",
	cyan11: "#107d98",
	cyan12: "#0d3c48"
};
/** Cyan dark scale. */
var cyanDark = {
	cyan1: "#0b161a",
	cyan2: "#101b20",
	cyan3: "#082c36",
	cyan4: "#003848",
	cyan5: "#004558",
	cyan6: "#045468",
	cyan7: "#12677e",
	cyan8: "#11809c",
	cyan9: "#00a2c7",
	cyan10: "#23afd0",
	cyan11: "#4ccce6",
	cyan12: "#b6ecf7"
};
/** Teal light scale. */
var teal = {
	teal1: "#fafefd",
	teal2: "#f3fbf9",
	teal3: "#e0f8f3",
	teal4: "#ccf3ea",
	teal5: "#b8eae0",
	teal6: "#a1ded2",
	teal7: "#83cdc1",
	teal8: "#53b9ab",
	teal9: "#12a594",
	teal10: "#0d9b8a",
	teal11: "#008573",
	teal12: "#0d3d38"
};
/** Teal dark scale. */
var tealDark = {
	teal1: "#0d1514",
	teal2: "#111c1b",
	teal3: "#0d2d2a",
	teal4: "#023b37",
	teal5: "#084843",
	teal6: "#145750",
	teal7: "#1c6961",
	teal8: "#207e73",
	teal9: "#12a594",
	teal10: "#0eb39e",
	teal11: "#0bd8b6",
	teal12: "#adf0dd"
};
/** Jade light scale. */
var jade = {
	jade1: "#fbfefd",
	jade2: "#f4fbf7",
	jade3: "#e6f7ed",
	jade4: "#d6f1e3",
	jade5: "#c3e9d7",
	jade6: "#acdec8",
	jade7: "#8bceb6",
	jade8: "#56ba9f",
	jade9: "#29a383",
	jade10: "#26997b",
	jade11: "#208368",
	jade12: "#1d3b31"
};
/** Jade dark scale. */
var jadeDark = {
	jade1: "#0d1512",
	jade2: "#121c18",
	jade3: "#0f2e22",
	jade4: "#0b3b2c",
	jade5: "#114837",
	jade6: "#1b5745",
	jade7: "#246854",
	jade8: "#2a7e68",
	jade9: "#29a383",
	jade10: "#27b08b",
	jade11: "#1fd8a4",
	jade12: "#adf0d4"
};
/** Green light scale. */
var green = {
	green1: "#fbfefc",
	green2: "#f4fbf6",
	green3: "#e6f6eb",
	green4: "#d6f1df",
	green5: "#c4e8d1",
	green6: "#adddc0",
	green7: "#8eceaa",
	green8: "#5bb98b",
	green9: "#30a46c",
	green10: "#2b9a66",
	green11: "#218358",
	green12: "#193b2d"
};
/** Green dark scale. */
var greenDark = {
	green1: "#0e1512",
	green2: "#121b17",
	green3: "#132d21",
	green4: "#113b29",
	green5: "#174933",
	green6: "#20573e",
	green7: "#28684a",
	green8: "#2f7c57",
	green9: "#30a46c",
	green10: "#33b074",
	green11: "#3dd68c",
	green12: "#b1f1cb"
};
/** Grass light scale. */
var grass = {
	grass1: "#fbfefb",
	grass2: "#f5fbf5",
	grass3: "#e9f6e9",
	grass4: "#daf1db",
	grass5: "#c9e8ca",
	grass6: "#b2ddb5",
	grass7: "#94ce9a",
	grass8: "#65ba74",
	grass9: "#46a758",
	grass10: "#3e9b4f",
	grass11: "#2a7e3b",
	grass12: "#203c25"
};
/** Grass dark scale. */
var grassDark = {
	grass1: "#0e1511",
	grass2: "#141a15",
	grass3: "#1b2a1e",
	grass4: "#1d3a24",
	grass5: "#25482d",
	grass6: "#2d5736",
	grass7: "#366740",
	grass8: "#3e7949",
	grass9: "#46a758",
	grass10: "#53b365",
	grass11: "#71d083",
	grass12: "#c2f0c2"
};
/** Brown light scale. */
var brown = {
	brown1: "#fefdfc",
	brown2: "#fcf9f6",
	brown3: "#f6eee7",
	brown4: "#f0e4d9",
	brown5: "#ebdaca",
	brown6: "#e4cdb7",
	brown7: "#dcbc9f",
	brown8: "#cea37e",
	brown9: "#ad7f58",
	brown10: "#a07553",
	brown11: "#815e46",
	brown12: "#3e332e"
};
/** Brown dark scale. */
var brownDark = {
	brown1: "#12110f",
	brown2: "#1c1816",
	brown3: "#28211d",
	brown4: "#322922",
	brown5: "#3e3128",
	brown6: "#4d3c2f",
	brown7: "#614a39",
	brown8: "#7c5f46",
	brown9: "#ad7f58",
	brown10: "#b88c67",
	brown11: "#dbb594",
	brown12: "#f2e1ca"
};
/** Bronze light scale. */
var bronze = {
	bronze1: "#fdfcfc",
	bronze2: "#fdf7f5",
	bronze3: "#f6edea",
	bronze4: "#efe4df",
	bronze5: "#e7d9d3",
	bronze6: "#dfcdc5",
	bronze7: "#d3bcb3",
	bronze8: "#c2a499",
	bronze9: "#a18072",
	bronze10: "#957468",
	bronze11: "#7d5e54",
	bronze12: "#43302b"
};
/** Bronze dark scale. */
var bronzeDark = {
	bronze1: "#141110",
	bronze2: "#1c1917",
	bronze3: "#262220",
	bronze4: "#302a27",
	bronze5: "#3b3330",
	bronze6: "#493e3a",
	bronze7: "#5a4c47",
	bronze8: "#6f5f58",
	bronze9: "#a18072",
	bronze10: "#ae8c7e",
	bronze11: "#d4b3a5",
	bronze12: "#ede0d9"
};
/** Gold light scale. */
var gold = {
	gold1: "#fdfdfc",
	gold2: "#faf9f2",
	gold3: "#f2f0e7",
	gold4: "#eae6db",
	gold5: "#e1dccf",
	gold6: "#d8d0bf",
	gold7: "#cbc0aa",
	gold8: "#b9a88d",
	gold9: "#978365",
	gold10: "#8c7a5e",
	gold11: "#71624b",
	gold12: "#3b352b"
};
/** Gold dark scale. */
var goldDark = {
	gold1: "#121211",
	gold2: "#1b1a17",
	gold3: "#24231f",
	gold4: "#2d2b26",
	gold5: "#38352e",
	gold6: "#444039",
	gold7: "#544f46",
	gold8: "#696256",
	gold9: "#978365",
	gold10: "#a39073",
	gold11: "#cbb99f",
	gold12: "#e8e2d9"
};
/** Sky light scale. */
var sky = {
	sky1: "#f9feff",
	sky2: "#f1fafd",
	sky3: "#e1f6fd",
	sky4: "#d1f0fa",
	sky5: "#bee7f5",
	sky6: "#a9daed",
	sky7: "#8dcae3",
	sky8: "#60b3d7",
	sky9: "#7ce2fe",
	sky10: "#74daf8",
	sky11: "#00749e",
	sky12: "#1d3e56"
};
/** Sky dark scale. */
var skyDark = {
	sky1: "#0d141f",
	sky2: "#111a27",
	sky3: "#112840",
	sky4: "#113555",
	sky5: "#154467",
	sky6: "#1b537b",
	sky7: "#1f6692",
	sky8: "#197cae",
	sky9: "#7ce2fe",
	sky10: "#a8eeff",
	sky11: "#75c7f0",
	sky12: "#c2f3ff"
};
/** Mint light scale. */
var mint = {
	mint1: "#f9fefd",
	mint2: "#f2fbf9",
	mint3: "#ddf9f2",
	mint4: "#c8f4e9",
	mint5: "#b3ecde",
	mint6: "#9ce0d0",
	mint7: "#7ecfbd",
	mint8: "#4cbba5",
	mint9: "#86ead4",
	mint10: "#7de0cb",
	mint11: "#027864",
	mint12: "#16433c"
};
/** Mint dark scale. */
var mintDark = {
	mint1: "#0e1515",
	mint2: "#0f1b1b",
	mint3: "#092c2b",
	mint4: "#003a38",
	mint5: "#004744",
	mint6: "#105650",
	mint7: "#1e685f",
	mint8: "#277f70",
	mint9: "#86ead4",
	mint10: "#a8f5e5",
	mint11: "#58d5ba",
	mint12: "#c4f5e1"
};
/** Lime light scale. */
var lime = {
	lime1: "#fcfdfa",
	lime2: "#f8faf3",
	lime3: "#eef6d6",
	lime4: "#e2f0bd",
	lime5: "#d3e7a6",
	lime6: "#c2da91",
	lime7: "#abc978",
	lime8: "#8db654",
	lime9: "#bdee63",
	lime10: "#b0e64c",
	lime11: "#5c7c2f",
	lime12: "#37401c"
};
/** Lime dark scale. */
var limeDark = {
	lime1: "#11130c",
	lime2: "#151a10",
	lime3: "#1f2917",
	lime4: "#29371d",
	lime5: "#334423",
	lime6: "#3d522a",
	lime7: "#496231",
	lime8: "#577538",
	lime9: "#bdee63",
	lime10: "#d4ff70",
	lime11: "#bde56c",
	lime12: "#e3f7ba"
};
/** Yellow light scale. */
var yellow = {
	yellow1: "#fdfdf9",
	yellow2: "#fefce9",
	yellow3: "#fffab8",
	yellow4: "#fff394",
	yellow5: "#ffe770",
	yellow6: "#f3d768",
	yellow7: "#e4c767",
	yellow8: "#d5ae39",
	yellow9: "#ffe629",
	yellow10: "#ffdc00",
	yellow11: "#9e6c00",
	yellow12: "#473b1f"
};
/** Yellow dark scale. */
var yellowDark = {
	yellow1: "#14120b",
	yellow2: "#1b180f",
	yellow3: "#2d2305",
	yellow4: "#362b00",
	yellow5: "#433500",
	yellow6: "#524202",
	yellow7: "#665417",
	yellow8: "#836a21",
	yellow9: "#ffe629",
	yellow10: "#ffff57",
	yellow11: "#f5e147",
	yellow12: "#f6eeb4"
};
/** Amber light scale. */
var amber = {
	amber1: "#fefdfb",
	amber2: "#fefbe9",
	amber3: "#fff7c2",
	amber4: "#ffee9c",
	amber5: "#fbe577",
	amber6: "#f3d673",
	amber7: "#e9c162",
	amber8: "#e2a336",
	amber9: "#ffc53d",
	amber10: "#ffba18",
	amber11: "#ab6400",
	amber12: "#4f3422"
};
/** Amber dark scale. */
var amberDark = {
	amber1: "#16120c",
	amber2: "#1d180f",
	amber3: "#302008",
	amber4: "#3f2700",
	amber5: "#4d3000",
	amber6: "#5c3d05",
	amber7: "#714f19",
	amber8: "#8f6424",
	amber9: "#ffc53d",
	amber10: "#ffd60a",
	amber11: "#ffca16",
	amber12: "#ffe7b3"
};
/** Orange light scale. */
var orange = {
	orange1: "#fefcfb",
	orange2: "#fff7ed",
	orange3: "#ffefd6",
	orange4: "#ffdfb5",
	orange5: "#ffd19a",
	orange6: "#ffc182",
	orange7: "#f5ae73",
	orange8: "#ec9455",
	orange9: "#f76b15",
	orange10: "#ef5f00",
	orange11: "#cc4e00",
	orange12: "#582d1d"
};
/** Orange dark scale. */
var orangeDark = {
	orange1: "#17120e",
	orange2: "#1e160f",
	orange3: "#331e0b",
	orange4: "#462100",
	orange5: "#562800",
	orange6: "#66350c",
	orange7: "#7e451d",
	orange8: "#a35829",
	orange9: "#f76b15",
	orange10: "#ff801f",
	orange11: "#ffa057",
	orange12: "#ffe0c2"
};
/** Neutral alpha scales for shadows. */
var ALPHA = {
	gray: [grayA, grayDarkA],
	mauve: [mauveA, mauveDarkA],
	slate: [slateA, slateDarkA],
	sage: [sageA, sageDarkA],
	olive: [oliveA, oliveDarkA],
	sand: [sandA, sandDarkA]
};
/** Black alpha steps for shadows. */
var blackAlpha = blackA;
/** Light and dark scales used by the theme. */
var PALETTES = {
	gray: [gray, grayDark],
	mauve: [mauve, mauveDark],
	slate: [slate, slateDark],
	sage: [sage, sageDark],
	olive: [olive, oliveDark],
	sand: [sand, sandDark],
	tomato: [tomato, tomatoDark],
	red: [red, redDark],
	ruby: [ruby, rubyDark],
	crimson: [crimson, crimsonDark],
	pink: [pink, pinkDark],
	plum: [plum, plumDark],
	purple: [purple, purpleDark],
	violet: [violet, violetDark],
	iris: [iris, irisDark],
	indigo: [indigo, indigoDark],
	blue: [blue, blueDark],
	cyan: [cyan, cyanDark],
	teal: [teal, tealDark],
	jade: [jade, jadeDark],
	green: [green, greenDark],
	grass: [grass, grassDark],
	brown: [brown, brownDark],
	bronze: [bronze, bronzeDark],
	gold: [gold, goldDark],
	sky: [sky, skyDark],
	mint: [mint, mintDark],
	lime: [lime, limeDark],
	yellow: [yellow, yellowDark],
	amber: [amber, amberDark],
	orange: [orange, orangeDark]
};
/** Resolve a neutral alpha step in both appearances. */
function paletteAlpha(palette, step) {
	const [light, dark] = ALPHA[palette];
	const key = `${palette}A${step}`;
	if (!light[key] || !dark[key]) throw new RangeError(`Unknown palette step: ${key}`);
	return `light-dark(${light[key]}, ${dark[key]})`;
}
/** Resolve a palette step in both appearances. */
function paletteColor(palette, step) {
	const [light, dark] = PALETTES[palette];
	const key = `${palette}${step}`;
	if (!light[key] || !dark[key]) throw new RangeError(`Unknown palette step: ${key}`);
	return `light-dark(${light[key]}, ${dark[key]})`;
}
/** Select text for solid palette backgrounds. */
function paletteForeground(palette) {
	switch (palette) {
		case "sky": return "#1c2024";
		case "mint": return "#1a211e";
		case "lime": return "#1d211c";
		case "yellow":
		case "amber": return "#21201c";
		default: return "white";
	}
}
/** Create scoped CSS variables without accessing the document or preferences. */
function createTheme(options = {}) {
	const accent = options.accent ?? "indigo";
	const gray = options.gray ?? "slate";
	const neutral = (step) => paletteColor(gray, step);
	const primary = (step) => paletteColor(accent, step);
	const scaling = Number.parseInt(options.scaling ?? "100%", 10) / 100;
	const roles = {
		background: neutral(1),
		foreground: neutral(12),
		card: neutral(2),
		cardForeground: neutral(12),
		popover: neutral(2),
		popoverForeground: neutral(12),
		primary: primary(9),
		primaryForeground: paletteForeground(accent),
		secondary: neutral(3),
		secondaryForeground: neutral(12),
		muted: neutral(3),
		mutedForeground: neutral(11),
		accent: primary(3),
		accentForeground: primary(12),
		border: neutral(6),
		input: neutral(7),
		ring: primary(8),
		destructive: paletteColor("red", 9),
		sidebar: neutral(2),
		sidebarForeground: neutral(12),
		sidebarPrimary: primary(9),
		sidebarPrimaryForeground: paletteForeground(accent),
		sidebarAccent: primary(3),
		sidebarAccentForeground: primary(12),
		sidebarBorder: neutral(6),
		sidebarRing: primary(8)
	};
	const style = {
		"color-scheme": options.appearance === void 0 || options.appearance === "system" ? "light dark" : options.appearance,
		"--destack-scaling": String(scaling)
	};
	for (const [name, value] of Object.entries(roles)) style[`--destack-color-${name}`] = value;
	for (let step = 1; step <= 12; step++) {
		style[`--destack-gray-${step}`] = neutral(step);
		style[`--destack-gray-a${step}`] = paletteAlpha(gray, step);
		style[`--destack-black-a${step}`] = blackAlpha[`blackA${step}`];
	}
	if (options.fontFamily !== void 0) style["--destack-default-font-family"] = options.fontFamily;
	if (options.monospaceFontFamily !== void 0) style["--destack-code-font-family"] = options.monospaceFontFamily;
	return {
		"data-destack-theme": options.appearance ?? "system",
		"data-radius": options.radius ?? "medium",
		style
	};
}
var _tmpl$$1 = "<h1 class=\"x1ywg3o6\">Hello Destack</h1>";
var _tmpl$2 = ["<button>Count: <!--$-->", "<!--/--></button>"];
/** Shared styles compiled into the browser stylesheet. */
/** Theme shared by the server and browser renders. */
var theme = createTheme({
	appearance: "light",
	accent: "indigo"
});
/** Render the build fixture. */
function App() {
	var _v$;
	const [count, setCount] = createSignal(0);
	onSettled(() => {
		document.addEventListener("click", focusHeading);
		return () => document.removeEventListener("click", focusHeading);
	});
	return ssrElement("main", theme, () => {
		return [ssr(_tmpl$$1), (_v$ = ssrScope(() => {
			return escape(count());
		}), ssr(_tmpl$2, _v$))];
	}, true);
}
/** Focus the heading from a browser lifecycle callback. */
function focusHeading() {
	document.querySelector("h1")?.focus();
}
var _tmpl$ = [
	"<span",
	" style=\"font-size:1.5em;text-align:center;position:fixed;left:0;bottom:55%;width:100%\">",
	"</span>"
];
function ErrorFallback(props) {
	console.error(props.error());
	httpStatus(500);
	return ssr(_tmpl$, ssrHydrationKey(), "500 | Internal Server Error");
}
function DefaultErrorBoundary(props) {
	return Errored({
		fallback: (error) => ErrorFallback({ error }),
		get children() {
			return props.children;
		}
	});
}
function render(request, context) {
	return renderToStream(() => DefaultErrorBoundary({ get children() {
		return Document({ get children() {
			return DefaultErrorBoundary({ get children() {
				return App({});
			} });
		} });
	} }), { manifest: _virtual_solid_manifest_default });
}
function joinAssetPath(base, file) {
	if (typeof base !== "string" || !base) base = "/";
	if (base[base.length - 1] !== "/") base += "/";
	return base + (file[0] === "/" ? file.slice(1) : file);
}
var clientEntryUrl;
function resolveClientEntry() {
	if (clientEntryUrl !== void 0) return clientEntryUrl;
	clientEntryUrl = null;
	const stamped = _virtual_solid_manifest_default._entry && _virtual_solid_manifest_default[_virtual_solid_manifest_default._entry];
	if (stamped && stamped.file) return clientEntryUrl = joinAssetPath(_virtual_solid_manifest_default._base, stamped.file);
	for (const key in _virtual_solid_manifest_default) {
		const chunk = _virtual_solid_manifest_default[key];
		if (chunk && chunk.isEntry && chunk.file) {
			clientEntryUrl = joinAssetPath(_virtual_solid_manifest_default._base, chunk.file);
			break;
		}
	}
	return clientEntryUrl;
}
var runMiddleware = (request, next) => next(request);
function assertRenderMode(mode, source) {
	if (mode !== "stream" && mode !== "async") throw new Error("[@solidjs/vite-plugin] " + source + " must be 'stream' or 'async', got " + JSON.stringify(mode));
	return mode;
}
function resolveRenderMode(event, options) {
	if (options.renderMode !== void 0) return assertRenderMode(options.renderMode, "handleRequest options.renderMode");
	return "stream";
}
function escapeAttribute(value) {
	return value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
}
function createHtmlChunkTransform(clientEntry, extraHead, nonce) {
	const nonceAttr = nonce ? " nonce=\"" + escapeAttribute(nonce) + "\"" : "";
	let first = true;
	let injected = false;
	return (chunk) => {
		if (!injected && chunk.includes("</head>")) {
			injected = true;
			chunk = chunk.replace("</head>", (clientEntry ? "<script type=\"module\"" + nonceAttr + " src=\"" + clientEntry + "\" async><\/script>" : "") + "</head>");
		}
		if (first) {
			first = false;
			chunk = "<!DOCTYPE html>" + chunk;
		}
		return chunk;
	};
}
async function dispatchRequest(request, event, options) {
	const clientEntry = options.clientEntry || resolveClientEntry();
	const renderMode = await resolveRenderMode(event, options);
	let result = render(request, {
		clientEntry,
		...options.context
	});
	if (result && typeof result.pipe !== "function" && typeof result.then === "function") result = await result;
	if (renderMode === "async" && result && typeof result.then === "function") result = await result;
	if (result instanceof Response) return result;
	return createSSRResponse(result, event, {
		responseInit: options.responseInit,
		nonce: options.nonce,
		transformChunk: createHtmlChunkTransform(clientEntry, options.devHead, options.nonce)
	});
}
async function handleRequest(request, options = {}) {
	const event = createRequestEvent(request, options.event);
	return commitEventResponse(await provideRequestEvent(event, () => runMiddleware(request, (req) => dispatchRequest(req || request, event, options))), event);
}
var virtual_solid_ssr_handler_default = { fetch(request) {
	return handleRequest(request);
} };
export { virtual_solid_ssr_handler_default as default, handleRequest };

//# sourceMappingURL=server.js.map