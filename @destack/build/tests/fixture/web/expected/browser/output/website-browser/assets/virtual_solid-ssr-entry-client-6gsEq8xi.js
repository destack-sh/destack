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
var StatusError = class extends Error {
	source;
	constructor(r, o) {
		super(o instanceof Error ? o.message : String(o), { cause: o });
		this.source = r;
	}
};
/** Return the user's error from an internal status wrapper. */ function unwrapStatusError(r) {
	return r instanceof StatusError ? r.cause : r;
}
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
var REACTIVE_MANUAL_WRITE = 1024;
/**
* The pending recompute is a re-ask of the same question: `refresh()` dirtied
* the node while no tracked input changed value. Cleared whenever a real
* value-change notification arrives (`insertSubs`), and consumed by
* `recompute` into the node's `_reask` classification — a quiet (re-ask)
* pending window does not read as pending (question-scoped pending model).
*/ var REACTIVE_REASK = 2048;
/**
* A dependency write landed while this subscriber was mid-recompute — a
* nested pull committed beneath one of its reads (#3037). The heap refuses
* RECOMPUTING nodes, so recompute's tail consumes this latch and reschedules:
* values the pass read before the nested commit are stale. Only set for
* links validated this pass (gen-current): a write to an untouched link is
* either re-read later in the pass (fresh) or trimmed with it (not a dep).
*/ var REACTIVE_MISSED_WAKE = 4096;
var CONFIG_HAS_LANE = 1024;
/** Set on a computed when its first firewall child signal is installed
* (projection machinery). Gates markNode's firewall-children walk with one
* masked read of the always-present _config — the walk's old `_child` read
* moved into the cold extension (§12), and an unconditional `_x` deref per
* marked node measurably taxed the propagation hot path (diamond -22%). */ var CONFIG_FW_CHILDREN = 4096;
/** Sticky mark: an authoritative-view reader read this node PAST an active
* override. The ack shape — an authoritative arrival EQUAL to the override —
* rides paths that are deliberately silent under A17 (every ordinary reader
* sees the override, so an equal landing changes nothing for them). A marked
* node notifies those readers on such paths anyway, so the landed truth is
* seen without re-firing ordinary subscribers. Never cleared — only nodes an
* until() predicate observed mid-override pay. */ var CONFIG_AUTHORITATIVE_OBSERVED = 16384;
/** HELD truth (#3164): this node's staged `_pendingValue` is confirming
* truth riding a transaction that retains optimism, revealed only at that
* transaction's settle. Two arming sites, one meaning: the store fold
* (a landing staged into the retaining transaction) and until()'s
* flip-entanglement (a foreign carrier's staged write, stolen when it
* flipped the awaited predicate truthy). Until the reveal, ordinary
* readers — lane and speculative recomputes included — keep committed:
* the staging notified subscribers as a plain write, so without the mask
* a mid-hold recompute composes live optimism with the confirming truth,
* a frame no timeline contains (GabbeV's union tear). Authoritative
* readers (until()'s predicate) and latest() tunnel through — the
* exemption that keeps holds deadlock-free. Override-covered nodes never
* arm: the override is their display and its revert their notification
* (A17). Cleared at commit (the commit IS the reveal); subscribers masked
* during the hold are woken by finalizePureQueue's post-revert pass. */ var CONFIG_HELD_TRUTH = 1 << 17;
/** In-flight async node whose inputs were PUBLISHED while it was pending: a
* batch or transaction committed with the node still `STATUS_PENDING` (an
* unobserved flight, #3305), so the inputs are on screen and the node's
* committed `_value` is stale against them. Governs read()'s reveal
* carve-out: a stale (render) reader in some OTHER transaction may show a
* foreign-held pending node's committed value — parallel transactions, no
* entanglement — only while that value is coherent with the visible frame,
* i.e. while the flight's inputs are themselves held (unpublished) and not
* lane-revealed. Set by `commitPendingNodes`; cleared when the node next
* enters pending fresh (a new flight from a settled state). */ var CONFIG_INPUTS_PUBLISHED = 1 << 21;
var NOT_PENDING = {};
var NO_SNAPSHOT = {};
/**
* Stand-in stored in `_overrideValue` for an optimistic write of literal
* `undefined` (#2898). The slot doubles as the optimistic-node brand
* (`undefined` = not optimistic, `NOT_PENDING` = at rest), so the raw value
* would erase the node's optimistic identity: the write turns invisible and
* follow-up writes route off the optimistic path and commit permanently.
* Same shape as NO_SNAPSHOT. Sites that surface the override VALUE unwrap
* via `visibleOverrideValue`; slot identity tests stay raw.
*/ var OVERRIDE_UNDEFINED = {};
/** Unwrap an active override's stored value for surfacing to readers (#2898). */ function unwrapOverride(E) {
	return E === OVERRIDE_UNDEFINED ? void 0 : E;
}
var defaultContext = {};
/**
* Brand symbol used by `Refreshable<T>` values (projection stores, async
* memos) to expose their underlying computation to `refresh()`. Not part of
* the user-facing API.
*
* @internal
*/ var $REFRESH = Symbol("refresh");
var activeLanes = /* @__PURE__ */ new Set();
/**
* Union-find: find the root lane.
*/ function findLane(n) {
	while (n.rn) n = n.rn;
	return n;
}
/**
* Merge two lanes when their dependency graphs overlap.
*/ function mergeLanes(n, e) {
	n = findLane(n);
	e = findLane(e);
	if (n === e) return n;
	e.rn = n;
	for (const i of e.he) n.he.add(i);
	e.he.clear();
	n.tn[0].push(...e.tn[0]);
	n.tn[1].push(...e.tn[1]);
	e.tn[0].length = 0;
	e.tn[1].length = 0;
	return n;
}
/**
* Resolve a node's lane: follow union-find chain, verify active, clear if stale.
*/ function resolveLane(n) {
	const e = n.o?.Oe;
	if (!e) return void 0;
	const i = findLane(e);
	if (activeLanes.has(i)) return i;
	if (n.o !== null) n.o.Oe = void 0;
}
function resolveTransition(n) {
	if (hasActiveOverride(n) && n.o?.Ot) {
		const e = ext(n).Ot = currentTransition(n.o?.Ot);
		if (e.ft !== true) return e;
		if (n.o !== null) n.o.Ot = null;
	}
	return resolveLane(n)?.ge ?? n.ge;
}
/**
* Check if a node has an active optimistic override.
*/ function hasActiveOverride(n) {
	const e = n.o;
	return e !== null && e.be !== void 0 && e.be !== NOT_PENDING;
}
/**
* Assign or merge a lane onto a node. At convergence points (node already has
* a different active lane), merge unless the node has an active override.
*/ function assignOrMergeLane(n, e) {
	const i = findLane(e);
	const t = n.o?.Oe;
	if (t) {
		if (t.rn) {
			ext(n).Oe = e;
			n.T |= CONFIG_HAS_LANE;
			return;
		}
		const r = findLane(t);
		if (activeLanes.has(r)) {
			if (r !== i && !hasActiveOverride(n)) {
				if (i.an && findLane(i.an) === r) {
					ext(n).Oe = e;
					n.T |= CONFIG_HAS_LANE;
				} else if (r.an && findLane(r.an) === i);
				else mergeLanes(i, r);
			}
			return;
		}
	}
	ext(n).Oe = e;
	n.T |= CONFIG_HAS_LANE;
}
var transitions = /* @__PURE__ */ new Set();
var dirtyQueue = {
	eE: new Array(2e3).fill(void 0),
	tE: false,
	Ke: 0,
	EE: 0
};
var zombieQueue = {
	eE: new Array(2e3).fill(void 0),
	tE: false,
	Ke: 0,
	EE: 0
};
/** runHeap callback that discards a queued zombie recompute instead of running
* it: unlink pure recompute entries; strip just the recompute bit from dirtied
* height-adjust entries so their height work still happens. */ function cancelZombieRecompute(e) {
	if (e.ie & 16) e.ie &= -12;
	else {
		deleteFromHeap(e, zombieQueue);
		e.ie &= -4;
	}
}
var clock = 0;
var activeTransition = null;
var scheduled = false;
var halted = false;
var haltNotified = false;
var syncDepth = 0;
var projectionWriteActive = false;
/** > 0 while an action's generator body is on the stack (the synchronous
* slice between yields). Maintained by action.ts around `it.next()`. */ var actionStepDepth = 0;
var transientStoreNodes = /* @__PURE__ */ new Set();
function canUseSimpleSyncFlush(e) {
	const t = e.m;
	return transitions.size === 0 && activeLanes.size === 0 && e.Xt.length === 0 && t.it.length === 0 && t.A.length === 0 && t.En.size === 0 && transientStoreNodes.size === 0;
}
function sweepTransientStoreNodes() {
	if (transientStoreNodes.size === 0) return;
	for (const e of transientStoreNodes) {
		if (e.u !== null) {
			transientStoreNodes.delete(e);
			continue;
		}
		if (e.Ge !== NOT_PENDING) continue;
		if (e.o?.be !== void 0 && e.o?.be !== NOT_PENDING) continue;
		if (e.o?.t) continue;
		transientStoreNodes.delete(e);
		if (e.T & 262144) slotUnobservedHook(e);
		else e.o?.Ct?.();
	}
}
/**
* Ambient work IS a transaction: the global queue always carries one
* current-transaction-shaped batch (`globalQueue._batch`). With no transition
* active, registrations (pending commits, optimistic nodes, affects marks,
* optimistic stores) land in a plain ambient batch that the plain flush
* finalizes; when a transition initializes it adopts the ambient batch's
* contents and `_batch` becomes the transition itself, so later registrations
* land there directly — no per-field aliasing.
*/ function createBatch() {
	return {
		_e: clock,
		$t: [],
		Re: /* @__PURE__ */ new Map(),
		it: [],
		A: [],
		En: /* @__PURE__ */ new Set(),
		ue: [],
		ei: {
			ti: [[], []],
			Xt: []
		},
		ft: false,
		lt: /* @__PURE__ */ new Set(),
		Et: null
	};
}
function mergeTransitionState(e, t) {
	t.ft = e;
	e.ue.push(...t.ue);
	for (const i of activeLanes) if (i.ge === t) i.ge = e;
	if (t.it.length) {
		e.it.push(...t.it);
		t.it.length = 0;
	}
	if (t.A.length) {
		e.A.push(...t.A);
		t.A.length = 0;
	}
	for (const i of t.En) e.En.add(i);
	for (const [i, n] of t.Re) {
		let t = e.Re.get(i);
		if (!t) e.Re.set(i, t = /* @__PURE__ */ new Set());
		for (const e of n) t.add(e);
	}
	for (const i of t.lt) e.lt.add(i);
	if (t.Et) (e.Et ??= []).push(...t.Et);
}
function schedule() {
	if (halted) {
		notifyHalted();
		return;
	}
	if (scheduled) return;
	scheduled = true;
	if (!syncDepth && !globalQueue.sn && !projectionWriteActive) queueMicrotask(flush);
}
/**
* Permanently halts the reactive system. Called when a user error escapes
* every boundary — app state is undefined at that point, so scheduling stops
* entirely rather than limping along with a half-applied update.
*/ function haltReactivity(e) {
	if (halted) return;
	halted = true;
	let t = "[REACTIVITY_HALTED]";
	const i = e !== void 0 && globalThis.reportError;
	i || e === void 0 ? console.error(t) : console.error(t, e);
	i && i(e);
}
function notifyHalted() {
	if (haltNotified) return;
	haltNotified = true;
	console.error("[REACTIVITY_HALTED]");
}
var queueRunToken = 0;
var Queue = class {
	qe = null;
	ti = [[], []];
	Xt = [];
	ii = 0;
	created = clock;
	addChild(e) {
		this.Xt.push(e);
		e.qe = this;
	}
	removeChild(e) {
		const t = this.Xt.indexOf(e);
		if (t >= 0) {
			this.Xt.splice(t, 1);
			e.qe = null;
		}
	}
	notify(e, t, i, n) {
		if (this.qe) return this.qe.notify(e, t, i, n);
		return false;
	}
	run(e) {
		if (this.ti[e - 1].length) {
			const t = this.ti[e - 1];
			this.ti[e - 1] = [];
			runQueue(t, e);
		}
		const t = this.Xt;
		const i = ++queueRunToken;
		for (let n = 0; n < t.length;) {
			const r = t[n];
			if (r.ii !== i) {
				r.ii = i;
				r.run?.(e);
				if (t[n] !== r) {
					n = 0;
					continue;
				}
			}
			n++;
		}
	}
	enqueue(e, t) {
		if (e) {
			if (currentOptimisticLane) findLane(currentOptimisticLane).tn[e - 1].push(t);
			else this.ti[e - 1].push(t);
		}
		schedule();
	}
	stashQueues(e) {
		e.ti[0].push(...this.ti[0]);
		e.ti[1].push(...this.ti[1]);
		this.ti = [[], []];
		for (let t = 0; t < this.Xt.length; t++) {
			let i = this.Xt[t];
			let n = e.Xt[t];
			if (!n) {
				n = {
					ti: [[], []],
					Xt: []
				};
				e.Xt[t] = n;
			}
			i.stashQueues(n);
		}
	}
	restoreQueues(e) {
		this.ti[0].push(...e.ti[0]);
		this.ti[1].push(...e.ti[1]);
		for (let t = 0; t < e.Xt.length; t++) {
			const i = e.Xt[t];
			let n = this.Xt[t];
			if (n) n.restoreQueues(i);
		}
	}
};
var GlobalQueue = class GlobalQueue extends Queue {
	sn = false;
	m = createBatch();
	static Fe;
	static We;
	static ct;
	static ni = null;
	static p = null;
	static G = null;
	static M = null;
	static N = null;
	static kt = null;
	static Lt = null;
	static pe = null;
	static Ne = null;
	static ke = null;
	static un = null;
	static wt = null;
	static Wt = null;
	static jt = null;
	static st = null;
	static k = null;
	static ri = null;
	static si = null;
	static Bt = null;
	static fn = null;
	static cn = null;
	static dn = null;
	static In = null;
	/** Patch-channel optimistic drain (next/patch.ts): optimistic emissions
	* apply at lane-effect timing — visible in flight, unlike the regular
	* effect queues an action stashes. Injected; null when unused. */
	static ln = null;
	static Zt = null;
	static qt = null;
	/** Is the node routed through a LIVE lane (`resolveLane`)? read()'s reveal
	* carve-out asks before showing a foreign-held pending node's committed
	* value: a lane-derived flight's inputs are already revealed through the
	* lane (#3334). Gated on CONFIG_HAS_LANE, which only the engine sets. */
	static Mt = null;
	static Kt = null;
	static nt = null;
	static ot = null;
	/** Authoritative-view reader wakeup: installed by until() and refresh() before
	* their first read. Call sites are gated by CONFIG_AUTHORITATIVE_OBSERVED, which
	* only such a reader's carve-out read can set, so `!` invocations are safe once
	* the gate holds (#3303). */
	static Qt = null;
	static ut = null;
	/** A18 supersession (#3331): own-source truth `value` landed under an active
	* override. The engine decides whether the graph re-derives — the value
	* differs from the override and is not a stale (older-action) answer (mark
	* the node, demote its lane cascade, notify), or returns to it after an
	* earlier differing arrival (clear the mark, notify) — and owns the
	* authoritative-observer wake for a silent confirm. Installed with the
	* optimistic engine; only reachable on a node that has an override. */
	static ye = null;
	/** read()'s value for a TRACKED reader of a superseded node (#3331): the
	* staged truth, unless the reader is a stale (render) reader of another
	* transaction — then the displayed override, as it keeps a foreign
	* transaction's committed value over its staged write. */
	static Yt = null;
	/** setSignal's authoritative (projection-write) landing on an override-
	* covered node (#3331 store twin): stage the truth for its transaction's
	* commit whatever its relation to the committed value — a landing equal to
	* committed still differs from the override — then _supersedeOverride
	* decides. Installed with the optimistic engine; only reachable on a node
	* that has an override. */
	static zt = null;
	static Nn = null;
	flush() {
		if (this.sn) return;
		if (activeTransition === null && dirtyQueue.EE < dirtyQueue.Ke && this.ti[0].length === 0 && this.ti[1].length === 0 && this.Xt.length === 0 && canUseSimpleSyncFlush(this)) {
			this.sn = true;
			try {
				sweepDormant();
				commitPendingNodes();
			} finally {
				this.sn = false;
			}
			clock++;
			scheduled = dirtyQueue.EE >= dirtyQueue.Ke || this.ti[0].length !== 0 || this.ti[1].length !== 0 || this.m.$t.length !== 0;
			return;
		}
		this.sn = true;
		try {
			sweepDormant();
			runHeap(dirtyQueue, GlobalQueue.Fe);
			if (activeTransition) {
				if (!transitionComplete(activeTransition)) {
					const e = activeTransition;
					runHeap(zombieQueue, this.m === e ? cancelZombieRecompute : GlobalQueue.Fe);
					if (this.m === e) currentBatch = this.m = createBatch();
					if (activeLanes.size) {
						GlobalQueue.In(1);
						GlobalQueue.In(2);
					}
					this.stashQueues(e.ei);
					clock++;
					scheduled = dirtyQueue.EE >= dirtyQueue.Ke || this.m.$t.length > 0;
					reassignPendingTransition(e.$t);
					activeTransition = null;
					finalizePureQueue(null, true);
					return;
				}
				const t = activeTransition;
				const i = this.m;
				i !== t && i.$t.push(...t.$t);
				this.restoreQueues(t.ei);
				transitions.delete(t);
				activeTransition = null;
				reassignPendingTransition(i.$t);
				finalizePureQueue(t);
				if (i === t) {
					const e = createBatch();
					e.$t = i.$t;
					e.it = i.it;
					e.A = i.A;
					e.En = i.En;
					currentBatch = this.m = e;
				}
			} else if (canUseSimpleSyncFlush(this)) {
				commitPendingNodes();
				if (dirtyQueue.EE >= dirtyQueue.Ke) {
					runHeap(dirtyQueue, GlobalQueue.Fe);
					commitPendingNodes();
				}
			} else {
				if (transitions.size) runHeap(zombieQueue, GlobalQueue.Fe);
				finalizePureQueue();
			}
			clock++;
			scheduled = dirtyQueue.EE >= dirtyQueue.Ke || activeTransition !== null;
			activeLanes.size && GlobalQueue.In(1);
			this.run(1);
			activeLanes.size && GlobalQueue.In(2);
			this.run(2);
		} finally {
			this.sn = false;
		}
	}
	notify(e, t, i, n) {
		if (t & 1) {
			if (i & 1) {
				const t = n ?? e.o?._;
				if (t?.l) return true;
				if (t) {
					if (!activeTransition && !e.ge && currentBatch.$t.length) this.initTransition();
					if (activeTransition) {
						const i = t.source;
						let n = activeTransition.Re.get(i);
						if (!n) activeTransition.Re.set(i, n = /* @__PURE__ */ new Set());
						const r = n.size;
						n.add(e);
						if (n.size !== r) {
							schedule();
							GlobalQueue.si?.(activeTransition);
						}
					}
				}
			}
			return true;
		}
		return false;
	}
	initTransition(e) {
		if (e) {
			e = currentTransition(e);
			if (e.ft === true || e === activeTransition) return;
		}
		if (!e && activeTransition && activeTransition._e === clock) return;
		if (!activeTransition) activeTransition = e ?? createBatch();
		else if (e) {
			const t = activeTransition;
			mergeTransitionState(e, t);
			this.restoreQueues(t.ei);
			transitions.delete(t);
			activeTransition = e;
		}
		transitions.add(activeTransition);
		activeTransition._e = clock;
		const t = this.m;
		if (t !== activeTransition) {
			for (let e = 0; e < t.$t.length; e++) {
				const i = t.$t[e];
				i.ge = activeTransition;
				activeTransition.$t.push(i);
			}
			for (let e = 0; e < t.it.length; e++) {
				const i = t.it[e];
				i.ge = activeTransition;
				activeTransition.it.push(i);
			}
			if (t.A.length) activeTransition.A.push(...t.A);
			for (const e of t.En) activeTransition.En.add(e);
			if (t.lt.size) {
				for (const e of t.lt) activeTransition.lt.add(e);
				t.lt.clear();
			}
			currentBatch = this.m = activeTransition;
		}
		for (const e of activeLanes) if (!e.ge) e.ge = activeTransition;
		schedule();
	}
};
function queuePendingNode(e) {
	currentBatch.$t.push(e);
}
var reaskArmed = false;
/** §12d: bumped by every recompute and every new subscriber edge. A node's
* staged-rewrite skip is sound only while NOTHING recomputed or linked since
* its last notify — a mid-batch pull can clean a marked subscriber, and a
* skipped re-write would leave it stale. */ var notifyEpoch = 0;
function bumpNotifyEpoch() {
	notifyEpoch++;
}
/** Provenance of the work currently running (A18 supersession, #3331): the
* invocation sequence of the action whose ambient window this is — set by
* action() for each slice; the flush that ends the window clears it — or,
* inside an async landing, the sequence captured when that flight was
* registered (asyncWrite sets it for the landing's synchronous propagation,
* so a sync recompute downstream of the landing — an optimistic wrapper over
* the async source — derives under the flight's provenance, and flights it
* registers inherit it). 0 is mainline: no action, always the current
* question. An override stamps this at its write (`_overrideStamp`); an
* answer whose flight an OLDER action issued is a stale question the user
* has since changed — it holds silently to commit instead of superseding. A
* slow source must not leak back in over a newer intent. Transactions merge,
* so the transition object cannot say WHICH action asked; this can. */ var origin = 0;
function setOrigin(e) {
	const t = origin;
	origin = e;
	return t;
}
function insertSubs(e, t = false) {
	e.At = notifyEpoch;
	const i = e.T;
	const n = (i & 1024 ? e.o?.Oe : void 0) || currentOptimisticLane;
	const r = (i & 512) !== 0 && e.o?.ze !== void 0;
	const s = reaskArmed;
	for (let i = e.u; i !== null; i = i.Te) {
		const e = i.Ie;
		if (s) e.ie &= ~REACTIVE_REASK;
		if (e.ie & 4 && i.yt === e.tt && i !== e.et) e.ie |= REACTIVE_MISSED_WAKE;
		if (r && e.T & 8) {
			e.ie |= 256;
			continue;
		}
		if (t && n) {
			e.ie |= 128;
			assignOrMergeLane(e, n);
		} else if (t) {
			e.ie |= 128;
			if (e.o) e.o.Oe = void 0;
		}
		enqueueSub(e);
	}
}
function commitPendingNode(e) {
	const t = e;
	if (!t.oe) {
		if (e.Ge !== NOT_PENDING) {
			e.me = e.Ge;
			e.Ge = NOT_PENDING;
		}
		if (e.T & 256) GlobalQueue.un(e);
		return;
	}
	if (e.Ge !== NOT_PENDING) {
		e.me = e.Ge;
		e.Ge = NOT_PENDING;
		if (e.Ce && e.Ce !== 3) e.He = true;
		if (e.o) e.o.Ue = false;
	}
	t.Ae = false;
	t.ie &= ~REACTIVE_MANUAL_WRITE;
	if (!(t.S & 1)) t.S &= -5;
	else e.T |= CONFIG_INPUTS_PUBLISHED;
	if (t.o != null && (t.o.Xe !== null || t.o.Je !== null)) GlobalQueue.We(t, false, true);
	if (e.T & 256) GlobalQueue.un(e);
}
var storeCommitHook = null;
/** Held truth committed this finalize, awaiting its post-revert wake (see
* finalizePureQueue): the commit IS the reveal, but subscribers must not
* re-derive until the settling transaction's optimistic overrides have
* reverted — a commit-time wake recomputes them in the window where
* confirming truth is committed and the override still displays, a torn
* frame no timeline contains. */ var heldRevealed = [];
function commitPendingNodes() {
	const e = currentBatch.$t;
	for (let t = 0; t < e.length; t++) {
		const i = e[t];
		commitPendingNode(i);
		i.ge = null;
		if (i.T & 131072) {
			i.T &= ~CONFIG_HELD_TRUTH;
			heldRevealed.push(i);
		}
	}
	e.length = 0;
	storeCommitHook?.();
}
function finalizePureQueue(e = null, t = false) {
	const i = currentBatch;
	const n = !t;
	if (n) commitPendingNodes();
	if (!t && globalQueue.Xt.length) checkBoundaryChildren(globalQueue);
	const r = e?.Et;
	const s = n && (e ?? i).it.length !== 0;
	if (r && !s) {
		for (const e of r) if (!(e.ie & 64)) enqueueSub(e);
	}
	const o = dirtyQueue.EE >= dirtyQueue.Ke;
	if (o) runHeap(dirtyQueue, GlobalQueue.Fe);
	if (n) {
		if (currentBatch !== i) {
			if (e === null || e === i) return;
		} else if (o) commitPendingNodes();
		const t = e ?? i;
		if (t.it.length) GlobalQueue.fn(t.it);
		if (r && s) {
			for (const e of r) if (!(e.ie & 64)) enqueueSub(e);
			schedule();
		}
		if (t.lt.size) {
			for (const e of t.lt) {
				if (e.ie & 64) continue;
				enqueueSub(e);
			}
			t.lt.clear();
			schedule();
		}
		if (t.A.length) {
			GlobalQueue.G(t.A);
			if (globalQueue.Xt.length) checkBoundaryChildren(globalQueue);
		}
		if (t.En.size) GlobalQueue.ni(t.En, e);
		if (heldRevealed.length !== 0) {
			while (heldRevealed.length) insertSubs(heldRevealed.pop());
			if (dirtyQueue.EE >= dirtyQueue.Ke) {
				runHeap(dirtyQueue, GlobalQueue.Fe);
				commitPendingNodes();
			}
		}
		sweepTransientStoreNodes();
		if (activeLanes.size) GlobalQueue.dn(e);
	}
}
function checkBoundaryChildren(e) {
	for (const t of e.Xt) {
		t.se?.();
		checkBoundaryChildren(t);
	}
}
function reassignPendingTransition(e) {
	for (let t = 0; t < e.length; t++) e[t].ge = activeTransition;
}
var globalQueue = new GlobalQueue();
var currentBatch = globalQueue.m;
function flush(e) {
	if (actionStepDepth > 0) return e ? e() : void 0;
	if (e) {
		syncDepth++;
		try {
			return e();
		} finally {
			try {
				flush();
			} finally {
				syncDepth--;
			}
		}
	}
	if (globalQueue.sn) return;
	if (halted) return;
	while (scheduled || activeTransition) globalQueue.flush();
	origin = 0;
}
function runQueue(e, t) {
	for (let i = 0; i < e.length; i++) e[i](t);
}
function reporterBlocksSource(e, t) {
	if (e.ie & 96) return false;
	if (e.o?.le?.has(t)) return true;
	for (let i = e.fe; i; i = i.ae) {
		let e = i.Se;
		while (e) {
			if (e === t || e.ce === t) return true;
			e = e.o?.Gt;
		}
	}
	return !!(e.S & 1 && e.o?._ instanceof NotReadyError && e.o?._.source === t);
}
function transitionComplete(e) {
	if (e.ft) return true;
	if (e.ue.length) return false;
	let t = true;
	for (const [i, n] of e.Re) {
		let r = false;
		for (const e of n) {
			if (reporterBlocksSource(e, i)) {
				r = true;
				break;
			}
			n.delete(e);
		}
		if (!r) e.Re.delete(i);
		else if (i.S & 1 && i.o?._?.source === i) {
			t = false;
			break;
		}
	}
	if (t && GlobalQueue.cn?.(e)) t = false;
	t && (e.ft = true);
	return t;
}
function currentTransition(e) {
	while (e.ft && typeof e.ft === "object") e = e.ft;
	return e;
}
/**
* The live transition blocked on `source` — the one whose render reader
* observed it pending (INV-3 records the observation in whichever transaction
* was active when the reader was notified). The observation is a fact about
* the node, so a hold check must not assume it was recorded in the transaction
* it happens to hold — lanes merge across transactions (#2912), and a merged
* root's transaction knows nothing of the async its members' transactions
* observed (#3335). Null when nobody is waiting.
*/ function waitingTransition(e) {
	for (const t of transitions) if (t.Re.has(e)) return t;
	return null;
}
function runInTransition(e, t) {
	const i = activeTransition;
	try {
		activeTransition = currentTransition(e);
		return t();
	} finally {
		activeTransition = i;
	}
}
/** The queue a node belongs to, picked from its own zombie flag. */ function queueFor(e) {
	return e.ie & 32 ? zombieQueue : dirtyQueue;
}
/**
* Schedule one subscriber to re-run on the next flush: inserted into its own
* (zombie-flag-routed) heap with the `_min` cursor pulled down. Tracked
* effects ride the heap too — the heap visit is their (empty) compute phase,
* which hands the callback to the user queue once the pass has committed
* (see GlobalQueue._update, #3291).
*/ function enqueueSub(e) {
	const E = queueFor(e);
	if (E.Ke > e.Be) E.Ke = e.Be;
	insertIntoHeap(e, E);
}
function actualInsertIntoHeap(e, E) {
	const t = (e.qe?.gt ? e.qe.bt?.Be : e.qe?.Be) ?? -1;
	if (t >= e.Be) e.Be = t + 1;
	const n = e.Be;
	const I = E.eE[n];
	if (I === void 0) E.eE[n] = e;
	else {
		const E = I.Tt;
		E.Nt = e;
		e.Tt = E;
		I.Tt = e;
	}
	if (n > E.EE) E.EE = n;
}
function insertIntoHeap(e, E) {
	let t = e.ie;
	if (t & 1036) return;
	if (t & 1) e.ie = t & -4 | 10;
	else {
		e.ie = t | 8;
		if (E.tE) markNode(e);
	}
	if (!(t & 16)) actualInsertIntoHeap(e, E);
}
function insertIntoHeapHeight(e, E) {
	let t = e.ie;
	if (t & 1052) return;
	e.ie = t | 16;
	actualInsertIntoHeap(e, E);
}
function deleteFromHeap(e, E) {
	const t = e.ie;
	if (!(t & 24)) return;
	e.ie = t & -25;
	const n = e.Be;
	if (e.Tt === e) E.eE[n] = void 0;
	else {
		const t = e.Nt;
		const I = E.eE[n];
		const o = t ?? I;
		if (e === I) E.eE[n] = t;
		else e.Tt.Nt = t;
		o.Tt = e.Tt;
	}
	e.Tt = e;
	e.Nt = void 0;
}
function markHeap(e) {
	if (e.tE) return;
	e.tE = true;
	for (let E = 0; E <= e.EE; E++) for (let t = e.eE[E]; t !== void 0; t = t.Nt) if (t.ie & 8) markNode(t);
}
function markNode(e, E = 2) {
	const t = e.ie;
	if ((t & 3) >= E) return;
	e.ie = t & -4 | E;
	for (let E = e.u; E !== null; E = E.Te) markNode(E.Ie, 1);
	if (e.T & 4096) for (let E = e.o.i; E !== null; E = E.Ee) for (let e = E.u; e !== null; e = e.Te) markNode(e.Ie, 1);
}
function runHeap(e, E) {
	e.tE = false;
	for (e.Ke = 0; e.Ke <= e.EE; e.Ke++) {
		let t = e.eE[e.Ke];
		while (t !== void 0) {
			if (t.ie & 8) E(t);
			else adjustHeight(t, e);
			t = e.eE[e.Ke];
		}
	}
	e.EE = 0;
}
function adjustHeight(e, E) {
	deleteFromHeap(e, E);
	let t = e.Be;
	for (let E = e.fe; E; E = E.ae) {
		const e = E.Se;
		const n = e.ce || e;
		if (n.oe && n.Be >= t) t = n.Be + 1;
	}
	if (e.Be !== t) {
		e.Be = t;
		for (let E = e.u; E !== null; E = E.Te) insertIntoHeapHeight(E.Ie, queueFor(E.Ie));
	}
}
function markDisposal(e) {
	let t = e.Ye;
	while (t) {
		const e = t.ie;
		t.ie = e | 32;
		if (e & 24) {
			deleteFromHeap(t, e & 32 ? zombieQueue : dirtyQueue);
			if (e & 8) insertIntoHeap(t, zombieQueue);
			else insertIntoHeapHeight(t, zombieQueue);
		}
		markDisposal(t);
		t = t.Ze;
	}
}
function disposeChildren(e, t = false, n) {
	const i = e.ie;
	if (i & 64) return;
	if (t) {
		e.ie = i | 64;
		const t = e;
		if (t.o?.Le || t.o?.Qe) GlobalQueue.un(t);
	}
	if (t && e.oe && e.o !== null) e.o.Pe = null;
	let o = n ? e.o?.Xe ?? null : e.Ye;
	while (o) {
		const e = o.Ze;
		const t = o;
		t.T &= -33;
		deleteFromHeap(t, queueFor(t));
		clearDeps(t);
		disposeChildren(o, true);
		o = e;
	}
	if (n) {
		if (e.o !== null) e.o.Xe = null;
	} else {
		e.Ye = null;
		e.$e = 0;
	}
	if (t && !n && !(i & 32) && e.qe !== null && !(e.qe.ie & 64)) {
		const t = e.St;
		const n = e.Ze;
		if (t !== null) t.Ze = n;
		else e.qe.Ye = n;
		if (n !== null) n.St = t;
		e.St = null;
	}
	runDisposal(e, n);
	if (t && e.Ht) {
		const t = e.Ht;
		e.Ht = void 0;
		t();
	}
}
function runDisposal(e, t) {
	let n = t ? e.o?.Je : e.we;
	if (!n) return;
	if (Array.isArray(n)) for (let e = 0; e < n.length; e++) {
		const t = n[e];
		t.call(t);
	}
	else n.call(n);
	if (t) {
		if (e.o !== null) e.o.Je = null;
	} else e.we = null;
}
function childId(e, t) {
	let n = e;
	while (n.T & 4 && n.qe) n = n.qe;
	if (n.id != null) return formatId(n.id, t ? n.$e++ : n.$e);
	throw new Error("");
}
/**
* Allocates and returns the next stable child id for `owner`. Used by
* hydration plumbing and `createUniqueId`. Not part of the user-facing API.
*
* @internal
*/ function getNextChildId(e) {
	return childId(e, true);
}
/**
* The id a freshly-created node inherits: an explicit `options.id` wins;
* transparent nodes share their parent's id; otherwise the parent's next
* child id is consumed (or `undefined` outside an id-carrying tree).
*/ function inheritId(e, t, n) {
	return e?.id ?? (t ? n?.id : n?.id != null ? getNextChildId(n) : void 0);
}
function formatId(e, t) {
	const n = t.toString(36), i = n.length - 1;
	return e + (i ? String.fromCharCode(64 + i) : "") + n;
}
/**
* Returns the current reactive **owner** — the lifecycle node that the next
* `cleanup()` / `onCleanup()` / `createSignal()` etc. will be attached to.
*
* Returns `null` if called outside any owner. Capture the owner with
* `getOwner()` and re-enter it later with `runWithOwner(owner, fn)` to attach
* disposables created from a callback (event handler, async resolution, etc.)
* back to a component's lifecycle.
*
* @example
* ```ts
* function defer<T>(fn: () => T) {
*   const owner = getOwner();
*   queueMicrotask(() => runWithOwner(owner, fn));
* }
* ```
*/ function getOwner() {
	return context;
}
/**
* Low-level: registers `fn` as a disposal callback on the current owner.
* Most code should use `onCleanup()` from `solid-js`, which adds dev-mode
* checks. `cleanup()` is the unchecked primitive used by internals.
*/ function cleanup(e) {
	if (!context) return e;
	if (!context.we) context.we = e;
	else if (Array.isArray(context.we)) context.we.push(e);
	else context.we = [context.we, e];
	return e;
}
function disposeRootSelf(e = true) {
	disposeChildren(this, e);
}
/**
* Creates a fresh owner attached as a child of the current owner (or as a
* detached root if there is none). Used by framework internals to group
* cleanups; app code should use `createRoot()` (host a reactive scope outside
* a component) or `runWithOwner()` (re-enter a captured owner).
*
* @internal
*/ function createOwner(e) {
	const t = context;
	const n = e?.transparent ?? false;
	const i = {
		id: inheritId(e, n, t),
		T: n ? 4 : 0,
		gt: true,
		bt: t?.gt ? t.bt : t,
		Ye: null,
		Ze: null,
		St: null,
		we: null,
		C: t?.C ?? globalQueue,
		xe: t?.xe || defaultContext,
		$e: 0,
		o: null,
		qe: t,
		dispose: disposeRootSelf
	};
	if (t) {
		const e = t.Ye;
		if (e === null) t.Ye = i;
		else {
			i.Ze = e;
			e.St = i;
			t.Ye = i;
		}
	}
	return i;
}
/**
* Creates a detached reactive root. The callback receives a `dispose()`
* function which, when called, tears down every signal, memo, effect, and
* `onCleanup` registered inside the root.
*
* Use this to host long-lived reactive scopes outside of a component (custom
* controllers, app bootstrapping, tests). Inside a component, prefer
* letting Solid's component lifecycle own things.
*
* @example
* ```ts
* const dispose = createRoot(dispose => {
*   const [n, setN] = createSignal(0);
*   createEffect(() => n(), value => console.log(value));
*   setInterval(() => setN(x => x + 1), 1000);
*   return dispose;
* });
*
* // Later, to tear everything down:
* dispose();
* ```
*
* @description https://docs.solidjs.com/reference/reactive-utilities/create-root
*/ function createRoot(e, t) {
	const n = createOwner(t);
	return runWithOwner(n, () => e(() => n.dispose()));
}
function unlinkSubs(e) {
	const n = e.Se;
	const l = e.ae;
	const o = e.Te;
	const s = e.en;
	if (o !== null) o.en = s;
	else n.dt = s;
	if (s !== null) s.Te = o;
	else {
		n.u = o;
		if (o === null) {
			if (n.T & 262144) slotUnobservedHook(n);
			else n.o?.Ct?.();
			const e = n;
			e.oe && e.T & 32 && !(e.ie & 32) && !(e.S & 1) && unobserved(e);
		}
	}
	return l;
}
function trimStaleDeps(e) {
	const n = e.et;
	let l = n !== null ? n.ae : e.fe;
	if (l !== null) {
		do
			l = unlinkSubs(l);
		while (l !== null);
		if (n !== null) n.ae = null;
		else e.fe = null;
	}
}
function clearDeps(e) {
	let n = e.fe;
	if (!n) return;
	do
		n = unlinkSubs(n);
	while (n !== null);
	e.fe = null;
	e.et = null;
}
function unobserved(e) {
	deleteFromHeap(e, queueFor(e));
	clearDeps(e);
	disposeChildren(e, true);
}
/**
* Deferred dormancy for never-observed auto-dispose computeds (#3078).
*
* An untracked top-level read of a subscriber-less observation-lifecycle memo
* used to call unobserved() inline at the end of read(). That kept the leak
* closed (the compute links the memo into its deps' sub lists — without a
* teardown point a never-observed memo is retained by its sources forever;
* upstream alien-signals has exactly this retention), but it made reads
* destructive: each read disposed the node, the next read revived it with a
* full recompute in whatever ambient transition/lane context happened to be
* current, so consecutive reads could return different answers with no write
* in between.
*
* Instead, reads queue the node here and the scheduler sweeps at the top of
* the next flush (before runHeap, so a same-tick dirtying is reclaimed
* instead of recomputed). Reads become idempotent within a tick (the node
* stays alive and serves its cache, uniform with observed memos) while
* reclamation still happens within one microtask — the enqueue site arms
* schedule(), so a flush is guaranteed even when no other work is queued.
*/ var dormantNodes = /* @__PURE__ */ new Set();
function sweepDormant() {
	if (dormantNodes.size === 0) return;
	for (const e of dormantNodes) if (!e.u && e.T & 32 && !(e.S & 1) && !(e.ie & 96)) unobserved(e);
	dormantNodes.clear();
}
function link(e, n, l = false) {
	const o = n.et;
	if (o !== null && o.Se === e) {
		o.je &&= l;
		return;
	}
	let s = null;
	const t = n.ie & 4;
	if (t) {
		s = o !== null ? o.ae : n.fe;
		if (s !== null && s.Se === e) {
			s.yt = n.tt;
			n.et = s;
			s.je = l;
			return;
		}
	}
	const r = e.dt;
	if (r !== null && r.Ie === n && (!t || r.yt === n.tt)) {
		if (t) r.je &&= l;
		else r.je = l;
		return;
	}
	const u = n.et = e.dt = {
		Se: e,
		Ie: n,
		ae: s,
		en: r,
		Te: null,
		yt: n.tt,
		je: l
	};
	if (o !== null) o.ae = u;
	else n.fe = u;
	if (r !== null) r.Te = u;
	else e.u = u;
	bumpNotifyEpoch();
}
function addPendingSource(e, n) {
	if (e.o?.le?.has(n)) return false;
	(ext(e).le ??= /* @__PURE__ */ new Set()).add(n);
	return true;
}
function removePendingSource(e, n) {
	const t = e.o?.le;
	if (!t?.delete(n)) return false;
	if (!t.size) e.o.le = void 0;
	return true;
}
function clearPendingSources(e) {
	if (e.o !== null) e.o.le = void 0;
}
function retryReaches(e, n) {
	for (let t = e.fe; t; t = t.ae) {
		const e = t.Se.ce || t.Se;
		if (e === n || e.o?.le?.has(n)) return true;
	}
	return false;
}
/**
* A loading-window node hit an unready source (sync throw in recompute, or a
* NotReadyError-rejected flight): register for the source's settle — the
* settlePendingSource walk runs off `_pendingSources` + `_blocked` alone —
* with NO read-visible pending status, no downstream propagation, no
* transition, no lane registration. Commit #0 keeps serving.
*/ function parkLoadingWindow(e, n) {
	ext(e).de = true;
	if (n.source) addPendingSource(e, n.source);
	if (!(e.S & 2)) setPendingError(e, n.source, n);
}
function setPendingError(e, n, t) {
	if (!n) {
		if (e.o !== null) e.o._ = null;
		return;
	}
	if (t instanceof NotReadyError && t.source === n) {
		ext(e)._ = t;
		return;
	}
	const r = e.o?._;
	if (!(r instanceof NotReadyError) || r.source !== n) ext(e)._ = new NotReadyError(n);
}
function forEachDependent(e, n) {
	for (let t = e.u; t !== null; t = t.Te) n(t.Ie, t);
	for (let t = e.o?.i ?? null; t !== null; t = t.Ee) for (let e = t.u; e !== null; e = e.Te) n(e.Ie, e);
}
function releaseIfSettledUnobserved(e) {
	e.oe && e.T & 32 && !e.u && !(e.ie & 32) && !(e.S & 1) && unobserved(e);
}
function releaseSettledDependents(e) {
	let n;
	const t = /* @__PURE__ */ new Set();
	const visit = (e) => {
		if (t.has(e)) return;
		t.add(e);
		if (!e.u && e.T & 32) (n ??= []).push(e);
		forEachDependent(e, visit);
	};
	forEachDependent(e, visit);
	if (n) for (const e of n) releaseIfSettledUnobserved(e);
}
function settleErroredDependents(e, n) {
	let t = false;
	const r = /* @__PURE__ */ new Set();
	const visit = (e) => {
		if (r.has(e)) return;
		r.add(e);
		if (e.o?._ === n) {
			enqueueSub(e);
			t = true;
		}
		forEachDependent(e, visit);
	};
	forEachDependent(e, visit);
	if (t) schedule();
}
function settlePendingSource(e, n = e) {
	removePendingSource(e, n);
	let t = false;
	let r;
	const o = /* @__PURE__ */ new Set();
	const i = GlobalQueue.Ne;
	const settle = (s) => {
		if (o.has(s)) return;
		if (n !== e && retryReaches(s, n)) return;
		if (!removePendingSource(s, n)) return;
		o.add(s);
		s._e = clock;
		const l = s.o?.le?.values().next().value;
		const u = s.S & 2;
		if (l) {
			if (!u) setPendingError(s, l);
			i?.(s);
		} else {
			s.S &= -2;
			if (!u) setPendingError(s);
			i?.(s);
			if (s.o?.de) {
				enqueueSub(s);
				t = true;
			}
			if (s.o !== null) s.o.de = false;
			if (!s.u && s.T & 32) (r ??= []).push(s);
		}
		forEachDependent(s, settle);
	};
	forEachDependent(e, settle);
	if (r) for (const e of r) releaseIfSettledUnobserved(e);
	if (t) schedule();
}
function isThenable(e) {
	return e != null && typeof e === "object" && typeof e.then === "function";
}
/** Fire and clear a node's iterator-flight cancellation hook (#3122). */ function releaseFlightTeardown(e) {
	const n = e.o?.De;
	if (n != null) {
		e.o.De = null;
		n();
	}
}
function handleAsync(e, n, t) {
	let r = false;
	let o = false;
	if (typeof n === "object" && n !== null) untrack(() => {
		r = n[Symbol.asyncIterator];
		o = !r && isThenable(n);
	});
	if (!o && !r) {
		if (e.o !== null) e.o.Pe = null;
		e.Ae = false;
		return n;
	}
	ext(e).Pe = n;
	const i = origin;
	let s;
	const settleTransition = () => {
		let n = resolveTransition(e);
		if (e.o?.Oe) n = waitingTransition(e) ?? n;
		if (n && e.S & 4 && !currentTransition(n).Re.has(e)) {
			e.ge = null;
			return;
		}
		globalQueue.initTransition(n);
	};
	const handleError = (t) => {
		if (e.o?.Pe !== n) return;
		let r = t instanceof NotReadyError;
		if (r && e.Ae) {
			if (e.o !== null) e.o.Pe = null;
			parkLoadingWindow(e, t);
			e._e = clock;
			return;
		}
		settleTransition();
		notifyStatus(e, r ? 1 : 2, t);
		if (r) settlePendingSource(e);
		e._e = clock;
		if (!r) releaseSettledDependents(e);
	};
	const asyncWrite = (r, o) => {
		if (e.o?.Pe !== n) return;
		if (e.ie & 130) return;
		setOrigin(i);
		settleTransition();
		const s = !!(e.S & 4);
		const l = e.o?.Ue;
		trimStaleDeps(e);
		clearStatus(e);
		if (l) e.o.Ue = true;
		const u = resolveLane(e);
		if (u) u.he.delete(e);
		if (t) {
			try {
				t(r);
			} catch (e) {
				handleError(e);
				return;
			}
			if (s) clearStatus(e, true);
		} else if (e.o?.be !== void 0) {
			if (e.Ge === NOT_PENDING) queuePendingNode(e);
			e.Ge = r;
			GlobalQueue.pe?.(e, r);
			if (!hasActiveOverride(e)) insertSubs(e);
			else GlobalQueue.ye(e, r);
			e._e = clock;
		} else if (u) {
			const n = e.Ce;
			const t = e.me;
			const o = e.ve;
			try {
				if (!n && s || !o || !o(r, t)) {
					e.me = r;
					e._e = clock;
					GlobalQueue.pe?.(e, r);
					insertSubs(e, true);
				}
			} catch (n) {
				notifyStatus(e, 2, n);
			}
		} else try {
			setSignal(e, () => r);
		} catch (n) {
			notifyStatus(e, 2, n);
		}
		if (e.Ge === NOT_PENDING) {
			e.Ae = false;
			if (l) e.o.Ue = false;
		}
		settlePendingSource(e);
		schedule();
		flush();
		o?.();
	};
	const settleAutodispose = () => {
		if (e.T & 32 && !e.u && !(e.S & 1)) {
			unobserved(e);
			return true;
		}
		return false;
	};
	const consumeIterator = (t, r) => {
		const o = t[Symbol.asyncIterator]();
		let i = false;
		let l = false;
		let u = !r;
		const close = () => {
			if (l) return;
			l = true;
			try {
				const e = o.return?.();
				if (isThenable(e)) e.then(void 0, () => {});
			} catch {}
		};
		r ? r(close) : cleanup(close);
		ext(e).De = close;
		const iterateOrRelease = () => {
			if (!settleAutodispose()) iterate();
		};
		const iterate = () => {
			let t, r, f = false, a = false, c = true;
			const S = o.next();
			(isThenable(S) ? S : { then: (e) => void e(S) }).then((r) => {
				if (c && u) {
					t = r;
					f = true;
					if (r.done) l = true;
				} else if (e.o?.Pe !== n) return;
				else if (!r.done) {
					i = true;
					asyncWrite(r.value, iterateOrRelease);
				} else {
					l = true;
					if (i) {
						schedule();
						flush();
					} else asyncWrite(void 0);
					settleAutodispose();
				}
			}, (t) => {
				if (c && u) {
					r = t;
					a = true;
				} else if (e.o?.Pe === n) {
					l = true;
					handleError(t);
					settleAutodispose();
				}
			});
			c = false;
			if (a) {
				l = true;
				handleError(r);
				if (u) throw r;
				return true;
			}
			if (f && !t.done) {
				s = t.value;
				i = true;
				return iterate();
			}
			return f && t.done;
		};
		const f = iterate();
		u = false;
		return i || f;
	};
	let l = null;
	const flattenIfIterable = (e, n) => {
		let t = false;
		if (typeof e === "object" && e !== null) untrack(() => {
			t = e[Symbol.asyncIterator];
		});
		if (!t) return false;
		const r = consumeIterator(e, n);
		if (!n) l = r;
		return true;
	};
	if (o) {
		let t = false, r = false, o, i = true;
		const registerDeferredClose = (n) => {
			if (!e.we) e.we = n;
			else if (Array.isArray(e.we)) e.we.push(n);
			else e.we = [e.we, n];
		};
		n.then((r) => {
			if (i) {
				s = r;
				t = true;
			} else if (e.o?.Pe === n && !(e.ie & 64) && flattenIfIterable(r, registerDeferredClose));
			else {
				asyncWrite(r);
				settleAutodispose();
			}
		}, (e) => {
			if (i) {
				o = e;
				r = true;
			} else {
				handleError(e);
				settleAutodispose();
			}
		});
		i = false;
		if (r) {
			handleError(o);
			throw o;
		} else if (!t) {
			if (e.Ae) return e.me;
			globalQueue.initTransition(resolveTransition(e));
			throw new NotReadyError(context);
		} else if (!flattenIfIterable(s)) e.Ae = false;
	}
	if (r) flattenIfIterable(n);
	if (l !== null) {
		if (!l) {
			if (e.Ae) return e.me;
			globalQueue.initTransition(resolveTransition(e));
			throw new NotReadyError(context);
		}
		e.Ae = false;
	}
	return s;
}
function clearStatus(e, n = false) {
	if (e.o?.le) clearPendingSources(e);
	if (e.o?.de) {
		if (e.o !== null) e.o.de = false;
	}
	if (e.o !== null) e.o.Ue = false;
	e.S = n ? 0 : e.S & 4;
	if (e.o?._) setPendingError(e);
	if (e.o?.Le || e.o?.Qe) GlobalQueue.Ne(e);
	if (e.o?.i && e.T & 2048 && GlobalQueue.ke !== null) GlobalQueue.ke(e);
	const t = statusNotifierOf(e);
	if (t) t.call(e);
}
function notifyStatus(e, n, t, r, o) {
	if (n === 2 && !(t instanceof StatusError) && !(t instanceof NotReadyError)) t = new StatusError(e, t);
	const i = n === 1 && t instanceof NotReadyError ? t.source : void 0;
	const s = i === e;
	const l = n === 1 && e.o?.be !== void 0 && !s;
	const u = l && hasActiveOverride(e);
	if (!r) {
		if (n === 1 && i) {
			addPendingSource(e, i);
			if (!(e.S & 1)) e.T &= ~CONFIG_INPUTS_PUBLISHED;
			e.S = 1 | e.S & 4;
			setPendingError(e, i, t);
		} else {
			clearPendingSources(e);
			e.S = n | (n !== 2 ? e.S & 4 : 0);
			ext(e)._ = t;
		}
		GlobalQueue.Ne?.(e);
		if (e.o?.i && e.T & 2048 && GlobalQueue.ke !== null) GlobalQueue.ke(e);
	}
	if (o && !r) assignOrMergeLane(e, o);
	const f = r || u;
	const a = r || l ? void 0 : o;
	const c = statusNotifierOf(e);
	if (c) {
		if (r && n === 1) return;
		if (f) c.call(e, n, t);
		else c.call(e);
		return;
	}
	forEachDependent(e, (e, r) => {
		e._e = clock;
		if (n === 1 && i && !e.o?.le?.has(i) || n !== 1 && (e.o?._ !== t || e.o?.le)) {
			if (r.je && n !== 1 && !(t instanceof NotReadyError)) {
				enqueueSub(e);
				schedule();
				return;
			}
			if (!f && !e.ge) queuePendingNode(e);
			notifyStatus(e, n, t, f, a);
		}
	});
}
GlobalQueue.Fe = (e) => {
	if (e.Ce === 3) {
		deleteFromHeap(e, queueFor(e));
		e.He = true;
		e.C.enqueue(2, e.Ve);
	} else recompute(e);
};
GlobalQueue.We = disposeChildren;
var tracking = false;
var stale = false;
var pendingCheckActive = false;
var latestReadActive = false;
var context = null;
var currentOptimisticLane = null;
var snapshotCaptureActive = false;
var snapshotSources = null;
function ownerInSnapshotScope(e) {
	while (e) {
		if (e.Me) return true;
		e = e.qe;
	}
	return false;
}
function recompute(e, t = false) {
	bumpNotifyEpoch();
	const n = e.Ce;
	if (!t) {
		if (e.ge && (!n || activeTransition) && activeTransition !== e.ge) globalQueue.initTransition(e.ge);
		deleteFromHeap(e, queueFor(e));
		if (e.o !== null) {
			e.o.Pe = null;
			releaseFlightTeardown(e);
		}
		if (e.ge || n === 3) disposeChildren(e);
		else if (e.Ye !== null || e.we !== null) {
			markDisposal(e);
			const t = ext(e);
			t.Je = e.we;
			t.Xe = e.Ye;
			e.we = null;
			e.Ye = null;
			e.$e = 0;
		}
	}
	let i = !!(e.ie & 128);
	const l = (e.T & 128) !== 0 && e.o?.be !== NOT_PENDING && e.o?.be !== void 0;
	const u = !!(e.S & 4);
	const o = e.S & 2 ? e.o?._ : void 0;
	const s = e.S & 1 ? e.o?.le : void 0;
	const a = e.o?.le?.has(e);
	const r = (e.ie & REACTIVE_REASK) !== 0;
	const c = e.Ae;
	const _ = context;
	context = e;
	e.et = null;
	e.tt++;
	e.ie = 4;
	e._e = clock;
	let f = e.Ge === NOT_PENDING ? e.me : e.Ge;
	let E = e.Be;
	let I = false;
	let N = tracking;
	let T = currentOptimisticLane;
	tracking = true;
	const d = latestReadActive;
	latestReadActive = false;
	if (i) {
		const t = GlobalQueue.nt(e, true);
		if (t) currentOptimisticLane = t;
		else if (t === false) i = false;
	} else if (activeTransition && !t && activeTransition.it.length) {
		const t = GlobalQueue.nt(e, false);
		if (t) {
			i = true;
			currentOptimisticLane = t;
		}
	}
	const S = n && n !== 2;
	const A = stale;
	if (S) stale = true;
	if (n && activeTransition !== null && activeTransition.lt.size) activeTransition.lt.delete(e);
	try {
		if (e.T & 64) {
			f = e.oe(f);
			if (e.o !== null) e.o.Pe = null;
			e.Ae = false;
		} else {
			const t = e.o?.Pe;
			const n = e.oe(f);
			const i = typeof n === "object" && n !== null;
			const l = e.o?.Pe !== t;
			f = l || !i ? n : handleAsync(e, n);
			if (!l && !i) {
				if (e.o !== null) e.o.Pe = null;
				e.Ae = false;
			}
		}
		if (e.S !== 0 || e.o !== null) clearStatus(e, t);
		if (e.T & 1024 && e.o?.Oe) GlobalQueue.ut(e);
	} catch (t) {
		const n = t instanceof NotReadyError;
		if (n && e.Ae) parkLoadingWindow(e, t);
		else {
			if (n && currentOptimisticLane) GlobalQueue.ot(e);
			let i = false;
			if (n) {
				ext(e).de = true;
				if (GlobalQueue.st !== null) i = GlobalQueue.st(e, r);
			}
			notifyStatus(e, n ? 1 : 2, t, void 0, n ? e.o?.Oe : void 0);
			if (n && a && !e.o?.Pe) settlePendingSource(e);
			if (i) GlobalQueue.k(e);
		}
	} finally {
		tracking = N;
		latestReadActive = d;
		if (S) stale = A;
		I = (e.ie & REACTIVE_MISSED_WAKE) !== 0;
		e.ie = 0 | (t ? e.ie & 256 : 0);
		context = _;
	}
	if (!e.o?._) {
		trimStaleDeps(e);
		const r = l ? unwrapOverride(e.o?.be) : i || e.Ge === NOT_PENDING ? e.me : e.Ge;
		let _ = false;
		try {
			_ = !n && u || !e.ve || !e.ve(r, f);
		} catch (t) {
			notifyStatus(e, 2, t);
		}
		if (n && _) {
			e.He = !e.o?._;
			if (!t) {
				e.C.enqueue(n, e.rt ??= GlobalQueue.ct.bind(null, e));
				let t = e._t;
				if (t !== activeTransition) {
					e._t = activeTransition;
					if (t !== null && (t = currentTransition(t)) !== activeTransition && !t.ft) {
						(t.Et ??= []).push(e);
						if (activeTransition !== null) (activeTransition.Et ??= []).push(e);
					}
				}
			}
		}
		if (e.o?._);
		else if (_) {
			const u = l ? e.o?.be : void 0;
			if (t || n && (activeTransition !== e.ge || activeTransition === null || e.T & 32768) || i) {
				e.me = f;
				if (l && i) {
					ext(e).be = f === void 0 ? OVERRIDE_UNDEFINED : f;
					e.Ge = NOT_PENDING;
				}
			} else {
				e.Ge = f;
				if (c) e.Ae = true;
				if ((activeTransition || e.ge) && GlobalQueue.pe !== null) GlobalQueue.pe(e, f);
			}
			if (e.u !== null && (!l || i || e.o?.be !== u)) insertSubs(e, i || l);
			else if (l && !i && e.o.It !== clock) GlobalQueue.ye(e, f);
		} else if (l) {
			if (e.Ge === NOT_PENDING) queuePendingNode(e);
			e.Ge = f;
			if (c) e.Ae = true;
			GlobalQueue.ye(e, f);
		} else if (e.Be != E) for (let t = e.u; t !== null; t = t.Te) insertIntoHeapHeight(t.Ie, queueFor(t.Ie));
		if (!_ && !e.o?._) {
			if (o !== void 0) settleErroredDependents(e, o);
			if (s) {
				for (const t of s) if (t !== e) settlePendingSource(e, t);
			}
		}
		if (a && !(e.S & 5)) settlePendingSource(e);
	}
	currentOptimisticLane = T;
	(e.Ge !== NOT_PENDING || e.o !== null && (e.o.Xe !== null || e.o.Je !== null) || e.S & 5) && (!t || e.S & 1) && (!e.ge || l) && queuePendingNode(e);
	e.ge && n && activeTransition !== e.ge && runInTransition(e.ge, () => recompute(e));
	if (I) {
		enqueueSub(e);
		schedule();
	}
}
function updateIfNecessary(e) {
	if (e.ie & 68) return;
	if (e.ie & 1) for (let t = e.fe; t; t = t.ae) {
		const n = t.Se;
		const i = n.ce || n;
		if (i.oe) updateIfNecessary(i);
		if (e.ie & 2) break;
	}
	if (e.ie & 130 || e.o?._ && e._e < clock && !e.o?.Pe) recompute(e);
	e.ie = e.ie & 280;
}
function computed(e, t) {
	const n = t?.transparent ?? false;
	const i = t !== null && typeof t === "object" && "loadingValue" in t;
	const l = {
		id: inheritId(t, n, context),
		T: (n ? 4 : 0) | (t?.ownedWrite ? 1 : 0) | (!context || t?.lazy ? 32 : 0) | (t?.sync ? 64 : 0) | (t?.H ? 2 : 0) | (snapshotCaptureActive && ownerInSnapshotScope(context) ? 8 : 0),
		ve: t?.equals ?? isEqual,
		we: null,
		C: context?.C ?? globalQueue,
		xe: context?.xe ?? defaultContext,
		$e: 0,
		oe: e,
		me: i ? t.loadingValue : void 0,
		Be: 0,
		Nt: void 0,
		Tt: null,
		fe: null,
		et: null,
		tt: 0,
		u: null,
		dt: null,
		qe: context,
		Ze: null,
		St: null,
		Ye: null,
		ie: t?.lazy ? 512 : 0,
		S: i ? 0 : 4,
		_e: clock,
		Ge: NOT_PENDING,
		ge: null,
		At: -1,
		Ae: i,
		o: null
	};
	if (t?.unobserved) ext(l).Ct = t.unobserved;
	setupComputedNode(l, t);
	return l;
}
/** Lazily allocate a node's cold extension (ONE shape for signals and
* computeds — `_x` access stays monomorphic). Installers write through
* this; hot paths read `el._x?._field` gated by the _config presence bits.
* Never call ext() just to store a field's default. */ function ext(e) {
	return e.o ??= {
		be: void 0,
		Ot: void 0,
		It: 0,
		Rt: 0,
		Oe: void 0,
		Le: void 0,
		Qe: void 0,
		Gt: void 0,
		t: 0,
		Pe: null,
		De: null,
		_: void 0,
		de: void 0,
		le: void 0,
		h: void 0,
		Ue: false,
		i: null,
		Ct: void 0,
		ze: void 0,
		Je: null,
		Xe: null,
		Dt: void 0
	};
}
/**
* Build an Effect node with all effect-specific fields baked into a single object literal,
* so V8 sees the full hidden class shape at construction time. Effects always run in lazy
* mode (recompute is called explicitly by `effect()`), so we hardcode the lazy bits and skip
* the auto-dispose CONFIG bit (effect() previously cleared it post-construction).
*/ function createEffectNode(e, t, n, i, l) {
	const u = l?.transparent ?? false;
	const o = {
		id: inheritId(l, u, context),
		T: (u ? 4 : 0) | (l?.ownedWrite ? 1 : 0) | (l?.sync ? 64 : 0) | (l?.Pt ?? 0) | (snapshotCaptureActive && ownerInSnapshotScope(context) ? 8 : 0),
		ve: false,
		we: null,
		C: context?.C ?? globalQueue,
		xe: context?.xe ?? defaultContext,
		$e: 0,
		oe: e,
		me: void 0,
		Be: 0,
		Nt: void 0,
		Tt: null,
		fe: null,
		et: null,
		tt: 0,
		u: null,
		dt: null,
		qe: context,
		Ze: null,
		St: null,
		Ye: null,
		ie: 512,
		S: 4,
		_e: clock,
		Ge: NOT_PENDING,
		ge: null,
		At: -1,
		Ae: false,
		He: false,
		Ft: void 0,
		ht: t,
		vt: n,
		Ht: void 0,
		Ce: i,
		_t: null,
		o: null
	};
	if (l?.unobserved) ext(o).Ct = l.unobserved;
	setupComputedNode(o, lazyOptions);
	return o;
}
/**
* The shared status notifier for effect nodes, installed once by effect.ts
* at module evaluation (`this`-dispatched — one function serves every
* effect, so nodes never store it). Boundary computeds keep their own
* per-node channel on `_x._notifyStatus`, which takes precedence.
*/ var effectStatusNotify = null;
function setEffectStatusNotify(e) {
	effectStatusNotify = e;
}
/** Resolve a node's status notifier: an own `_x` channel (boundaries) wins;
* effect nodes (`_type` — EFFECT_PURE is 0, and only effect literals carry
* the field) fall back to the shared notifier. Presence doubles as the
* "display consumer" membership test in the status walks, exactly as the
* per-node field did when every effect carried one. */ function statusNotifierOf(e) {
	const t = e.o?.h;
	if (t !== void 0) return t;
	return e.Ce ? effectStatusNotify ?? void 0 : void 0;
}
var lazyOptions = { lazy: true };
function setupComputedNode(e, t) {
	e.Tt = e;
	const n = context?.gt ? context.bt : context;
	if (context) {
		const t = context.Ye;
		if (t === null) context.Ye = e;
		else {
			e.Ze = t;
			t.St = e;
			context.Ye = e;
		}
	}
	if (n) e.Be = n.Be + 1;
	if (GlobalQueue.kt !== null) GlobalQueue.kt(e);
	!t?.lazy && recompute(e, true);
	if (snapshotCaptureActive && !t?.lazy) {
		if (!(e.S & 1) && !(e.T & 2)) {
			ext(e).ze = e.me === void 0 ? NO_SNAPSHOT : e.me;
			e.T |= 512;
			snapshotSources.add(e);
		}
	}
}
function signal(e, t, n = null) {
	const i = {
		ve: t?.equals ?? isEqual,
		T: (t?.ownedWrite ? 1 : 0) | (t?.H ? 2 : 0),
		me: e,
		u: null,
		dt: null,
		_e: clock,
		ce: n,
		Ee: n?.o?.i || null,
		Vt: null,
		Ge: NOT_PENDING,
		ge: null,
		At: -1,
		o: null
	};
	if (t?.unobserved) ext(i).Ct = t.unobserved;
	if (n) linkFirewallChild(n, i);
	if (snapshotCaptureActive && !(i.T & 2) && !((n?.S ?? 0) & 1)) {
		ext(i).ze = e === void 0 ? NO_SNAPSHOT : e;
		i.T |= 512;
		snapshotSources.add(i);
	}
	return i;
}
/** The shared slot-node unobserved handler — a live binding read directly by
* the sweep sites (no wrapper frame, no null check: a CONFIG_SLOT_NODE node
* existing implies the store module loaded and registered the hook). */ var slotUnobservedHook;
/** Push a new node onto its firewall's child chain (the literal already
* points `_nextChild` at the old head). Doubly linked so a released leaf
* unlinks in O(1) — the chain is walked per mark of the projection and
* would otherwise grow by one node per leaf ever read (#3351). */ function linkFirewallChild(e, t) {
	const n = t.Ee;
	if (n !== null) n.Vt = t;
	ext(e).i = t;
	e.T |= CONFIG_FW_CHILDREN;
}
function isEqual(e, t) {
	return e === t;
}
/**
* Runs `fn` outside of any reactive tracking — reads inside `fn` will not
* subscribe the current scope. Returns whatever `fn` returns.
*
* Use `untrack` inside a memo or effect when you need to read a signal once
* without making the surrounding computation depend on its future changes.
*
* Pass a `strictReadLabel` string to enable a dev-mode warning: any reactive
* read inside `fn` that isn't inside a nested tracking scope will log a
* warning naming the label.
*
* @example
* ```ts
* createEffect(
*   () => trigger(),                 // tracks `trigger` only
*   () => {
*     const snapshot = untrack(() => state); // read once, untracked
*     log(snapshot);
*   }
* );
* ```
*/ function untrack(e, t) {
	if (GlobalQueue.Lt === null && !tracking && true) return e();
	const n = tracking;
	tracking = false;
	try {
		if (GlobalQueue.Lt !== null) return GlobalQueue.Lt(e);
		return e();
	} finally {
		tracking = n;
	}
}
/**
* Bring a computed to a readable state: lazy/disposed nodes are (re)computed;
* an isPending() probe (`refresh`) additionally pulls the node fully up to
* date so its status flags reflect the current graph.
*/ function prepareComputed(e, t) {
	if (e.ie & 512) {
		e.ie &= -513;
		recompute(e, true);
	} else if (e.ie & 64) {
		if (e.T & 32) recompute(e, true);
	} else if (t) updateIfNecessary(e);
}
/**
* Stale-reader term of the value selections below: a render effect reading a
* node some OTHER live transaction has staged sees the committed value. The
* commit is silent — the staging walk was the notification — so a reader
* that linked AFTER that walk (an effect created during the hold, a store
* key first read under it) would show the old value past the reveal: record
* it for the transaction's commit replay (the `_gatedSubs` contract lanes
* already use). An effect the transaction itself computed re-derives at its
* commit on its own (parked run, or the contested re-derive, #3322) and is
* not recorded — replaying it too would publish the frame twice.
*/ function heldFromStale(e, t) {
	const n = e.ge;
	if (n === null || n === activeTransition) return false;
	const i = currentTransition(n);
	const l = t._t;
	if (l == null || currentTransition(l) !== i) i.lt.add(t);
	return true;
}
function read(e) {
	if (latestReadActive) return GlobalQueue.wt(e);
	let t = context;
	if (t?.gt) t = t.bt;
	const n = e;
	const i = e.ce;
	const l = i || e;
	if (pendingCheckActive) GlobalQueue.Wt(e, t, l, i);
	else if (typeof n.oe === "function") prepareComputed(e, false);
	if (!n.oe && l === e && e.o?.be === void 0 && e.o?.ze === void 0 && activeTransition === null && currentOptimisticLane === null && !snapshotCaptureActive && true) {
		if (t && tracking) link(e, t);
		return !t || e.Ge === NOT_PENDING || t.T & 16 || stale && heldFromStale(e, t) ? e.me : e.Ge;
	}
	if (t && tracking) {
		link(e, t, pendingCheckActive);
		if (l.oe) {
			const n = queueFor(e);
			if (l.Be >= n.Ke) {
				markNode(t);
				markHeap(n);
				updateIfNecessary(l);
			} else if (t.T & 65536) updateIfNecessary(l);
			const i = l.Be;
			if (i >= t.Be && e.qe !== t) t.Be = i + 1;
		}
	}
	if (l.S & 1) {
		if (t && !(stale && !(l.S & 4) && !(l.T & 2097152) && !(l.T & 1024 && GlobalQueue.Mt(l)) && heldFromStale(l, t))) {
			if (currentOptimisticLane === null || GlobalQueue.qt(l)) {
				if (!tracking && e !== t) link(e, t);
				throw l.o?._;
			}
		} else if (!t && l.S & 4) throw l.o?._;
	}
	if (l.oe && l.S & 2) {
		if (tracking && !pendingCheckActive && l._e < clock) {
			recompute(l);
			return read(e);
		} else throw l.o?._;
	}
	if (snapshotCaptureActive && t && t.T & 8) {
		const n = e.o?.ze;
		if (n !== void 0) {
			const i = n === NO_SNAPSHOT ? void 0 : n;
			if ((e.Ge !== NOT_PENDING ? e.Ge : e.me) !== i) t.ie |= 256;
			return i;
		}
	}
	if (e.o?.be !== void 0 && e.o?.be !== NOT_PENDING) {
		if (!(t && t.T & 8192)) {
			if (t && e.T & 524288) return GlobalQueue.Yt(e);
			return unwrapOverride(e.o?.be);
		}
		e.T |= CONFIG_AUTHORITATIVE_OBSERVED;
	}
	if (currentOptimisticLane !== null && activeTransition !== null && t !== null && GlobalQueue.Zt(e, l, t)) return e.me;
	const u = !t || currentOptimisticLane !== null && GlobalQueue.Kt(e, l, t) || e.Ge === NOT_PENDING || t.T & 16 || stale && heldFromStale(e, t) || e.T & 131072 && !latestReadActive && !(t.T & 8192) ? e.me : e.Ge;
	if (pendingCheckActive) GlobalQueue.jt(e, u);
	if (!t && l === e && typeof n.oe === "function" && e.T & 32 && !(l.S & 1) && !e.u) {
		dormantNodes.add(e);
		schedule();
	}
	return u;
}
function setSignal(e, t) {
	if (e.ge && activeTransition !== e.ge) globalQueue.initTransition(e.ge);
	if (e.T & 128) {
		if (!projectionWriteActive) return GlobalQueue.Bt(e, t);
		const n = e.o?.be;
		if (n !== void 0 && n !== NOT_PENDING) return GlobalQueue.zt(e, t);
	}
	const n = e.Ge === NOT_PENDING ? e.me : e.Ge;
	if (typeof t === "function") t = t(n);
	if (!(!!(e.S & 4) || !e.ve || !e.ve(n, t))) return t;
	const l = e.Ge !== NOT_PENDING;
	if (!l) queuePendingNode(e);
	e.Ge = t;
	e.T & 256 && GlobalQueue.pe !== null && GlobalQueue.pe(e, t);
	if (e.oe !== void 0) e._e = clock;
	if (l && e.At === notifyEpoch && currentOptimisticLane === null && !reaskArmed) return t;
	insertSubs(e);
	schedule();
	return t;
}
/**
* Suppresses automatic recomputation of `el` until the scheduler drains. Used
* when a manual write should win over dependency changes queued in the same
* tick. The MANUAL_WRITE flag is cleared by the pending-node drain; projection
* computeds don't commit values, but they still need the same end-of-tick
* cleanup point.
*/ function suppressComputedRecompute(e) {
	deleteFromHeap(e, queueFor(e));
	if (!(e.ie & 1024) && e.Ge === NOT_PENDING) {
		queuePendingNode(e);
		schedule();
	}
	e.ie = e.ie & -4 | REACTIVE_MANUAL_WRITE;
	e.Jt = clock;
}
/**
* User-facing setter for the memo form of `createSignal(fn)`. Behaves like
* `setSignal`, but also cancels any pending recompute of the memo so the
* manual value wins over a value that would otherwise be produced by an
* upstream change in the same tick.
*/ function setMemo(e, t) {
	const n = setSignal(e, t);
	suppressComputedRecompute(e);
	return n;
}
/**
* Executes `fn` with the given `owner` set as the current owner. Any reactive
* primitives (`createSignal`, `createMemo`, `createEffect`, `onCleanup`,
* `cleanup`, etc.) created inside `fn` are attached to that owner, so they
* are disposed when the owner is disposed.
*
* The classic pattern: capture the current owner with `getOwner()` inside a
* component, then re-enter it from a callback (event handler, async resolve,
* setTimeout) so disposables created in the callback get cleaned up with the
* component.
*
* @example
* ```ts
* function delayed<T>(ms: number, fn: () => T) {
*   const owner = getOwner();
*   setTimeout(() => runWithOwner(owner, fn), ms);
* }
* ```
*/ function runWithOwner(e, t) {
	const n = context;
	const i = tracking;
	context = e;
	tracking = false;
	try {
		return t();
	} finally {
		context = n;
		tracking = i;
	}
}
function staleValues(e, t = true) {
	const n = stale;
	stale = t;
	try {
		return e();
	} finally {
		stale = n;
	}
}
/**
* Context provides a form of dependency injection. It is used to save from needing to pass
* data as props through intermediate components. This function creates a new context object
* that can be used with `getContext` and `setContext`.
*
* A default value can be provided here which will be used when a specific value is not provided
* via a `setContext` call.
*/ function createContext(e, t) {
	return {
		id: Symbol(t),
		defaultValue: e
	};
}
/**
* Low-level owner-targeted context read. The user-facing read API is
* `useContext` (in `solid-js`), which wraps this primitive. Exposed here for
* cross-package wiring (e.g. hydration-aware context plumbing).
*
* @throws `NoOwnerError` if there's no owner at the time of call.
* @throws `ContextNotFoundError` if a context value has not been set yet.
*
* @internal
*/ function getContext(e, t = getOwner()) {
	if (!t) throw new NoOwnerError();
	let r = t.xe[e.id];
	if (r === void 0) r = e.defaultValue;
	if (r === void 0) throw new ContextNotFoundError();
	return r;
}
/**
* Low-level owner-targeted context write. The user-facing API is
* `createContext` (in `solid-js`); its provider component wraps this
* primitive. Exposed here for cross-package wiring.
*
* @throws `NoOwnerError` if there's no owner at the time of call.
*
* @internal
*/ function setContext(e, t, r = getOwner()) {
	if (!r) throw new NoOwnerError();
	r.xe = {
		...r.xe,
		[e.id]: t === void 0 ? e.defaultValue : t
	};
}
/**
* Effects are the leaf nodes of our reactive graph. When their sources change, they are
* automatically added to the queue of effects to re-execute, which will cause them to fetch their
* sources and recompute
*/ function effect$1(t, e, E, r) {
	const f = createEffectNode(t, e, E, !!r?.user ? 2 : 1, r);
	recompute(f, true);
	!r?.defer && (f.Ce === 2 || r?.schedule ? f.C.enqueue(f.Ce, runEffect.bind(null, f)) : runEffect(f, 4));
}
function notifyEffectStatus(t, e) {
	const E = t !== void 0 ? t : this.S;
	const r = e !== void 0 ? e : this.o?._;
	if (E & 2) {
		this.C.notify(this, 1, 0);
		if (this.Ce === 2) {
			if (this.S & 2) {
				this.He = true;
				this.C.enqueue(this.Ce, this.rt ??= runEffect.bind(null, this));
			}
			return;
		}
		if (!this.C.notify(this, 2, 2)) {
			haltReactivity(unwrapStatusError(r));
			throw r;
		}
	} else if (this.Ce === 1) this.C.notify(this, 3, E, r);
}
function runEffect(t, e) {
	if (!t.He || t.ie & 64) return;
	if (t._t !== null && !currentTransition(t._t).ft && (e & 4 ? !t.o?.Oe : activeTransition !== null)) {
		t.C.enqueue(t.Ce, t.rt);
		return;
	}
	if (t.S & 2 && t.Ce === 2) {
		const e = unwrapStatusError(t.o?._);
		t.Ft = t.me;
		t.He = false;
		try {
			t.vt ? t.vt(e, () => {
				const e = t.Ht;
				t.Ht = void 0;
				e?.();
			}) : console.error(e);
		} catch (e) {
			if (!t.C.notify(t, 2, 2)) {
				haltReactivity(e);
				throw e;
			}
		}
		return;
	}
	const E = t.Ht;
	t.Ht = void 0;
	try {
		E?.();
		t.Ht = t.ht(t.me, t.Ft);
	} catch (e) {
		ext(t)._ = new StatusError(t, e);
		t.S |= 2;
		if (!t.C.notify(t, 2, 2)) {
			haltReactivity(e);
			throw e;
		}
	} finally {
		t.Ft = t.me;
		t.He = false;
	}
}
GlobalQueue.ct = runEffect;
/**
* Internal tracked effect - bypasses heap, goes directly to effect queue.
* Runs as a leaf owner: child primitives and onCleanup are forbidden (false throws).
* Uses stale reads.
*/ function trackedEffect(t, e) {
	const run = () => {
		if (!E.He || E.ie & 64) return;
		try {
			E.He = false;
			recompute(E);
		} finally {}
	};
	const E = computed(() => {
		const e = E.Ht;
		E.Ht = void 0;
		e?.();
		const r = staleValues(t);
		E.Ht = r;
	}, {
		...e,
		lazy: true
	});
	E.Ht = void 0;
	E.T = E.T & -33 | 16;
	E.He = true;
	E.Ce = 3;
	E.Ve = run;
	enqueueSub(E);
	schedule();
}
setEffectStatusNotify(notifyEffectStatus);
function accessor(e) {
	const t = read.bind(null, e);
	t[$REFRESH] = e;
	return t;
}
function createSignal$1(e, t) {
	if (typeof e === "function") {
		const n = computed(e, t);
		n.T &= -33;
		return [accessor(n), setMemo.bind(null, n)];
	}
	const n = signal(e, t);
	return [accessor(n), setSignal.bind(null, n)];
}
/**
* Creates a reactive computation that runs during the render phase as DOM elements
* are created and updated but not necessarily connected.
*
* Same compute / effect split as `createEffect`, but scheduled inside the render
* queue rather than after it. Reach for this only when authoring renderer
* plumbing (custom DOM bindings, JSX-generated `insert()` / `spread()` calls).
* App code should use `createEffect`.
*
* ```typescript
* createRenderEffect<T>(compute, effectFn, options?: EffectOptions);
* ```
* @param compute a function that receives its previous value and returns a new value used to react on a computation
* @param effectFn a function that receives the new value and is used to perform side effects
* @param options `EffectOptions` -- name, defer, schedule, transparent
*
* @example
* ```ts
* // Custom directive: bind an element's textContent to a reactive source.
* function bindText(el: HTMLElement, source: () => string) {
*   createRenderEffect(
*     () => source(),
*     value => { el.textContent = value; }
*   );
* }
* ```
*
* @description https://docs.solidjs.com/reference/secondary-primitives/create-render-effect
*/ function createRenderEffect$1(e, t, n) {
	effect$1(e, t, void 0, n);
}
/**
* Schedules `callback` to run **once** after the reactive graph has fully
* settled — i.e. once every pending async read inside the current owner has
* resolved and the queue has flushed. Each call registers a single fire; it
* does not create an ongoing subscription.
*
* The canonical lifecycle primitive in 2.0. Three main usages:
*
* - **Component-level setup-and-teardown** *(the most common shape)*: run
*   setup after the component's first stable render and **return a cleanup
*   function** to dispose it on owner disposal. This is the replacement for
*   the 1.x `onMount` + `onCleanup` pairing — setup and teardown live in one
*   block, and `onCleanup` is no longer the right tool for component
*   bodies. (`onMount` no longer exists in 2.0.)
* - **Post-settle "ready" hook:** run once after a component's first stable
*   render — analytics ping, focus, scroll-into-view, etc. No cleanup needed.
* - **Inside an event handler:** schedule work to run after the action /
*   transition triggered by the event has completed.
*
* Reactive reads inside the callback are *not* tracked — to react to
* subsequent settles, register a new `onSettled` each time.
*
* The callback runs during the settle flush itself, which gives it the same
* write semantics as every other effect-phase scope (the effect half of
* `createEffect`, event handlers):
*
* - **Writes** are queued into the same flush's continuation — dependent memos
*   and effects update before the flush returns — but reads inside the
*   callback keep returning the settled (pre-write) values. A callback never
*   observes its own unsettled write. Functional setters still compose:
*   `set(v => v + 1)` twice increments twice.
* - **`flush()` cannot be called** from inside the callback — the flush is
*   already running (dev throws; production is a no-op). To force a drain
*   after this settle, defer it: `queueMicrotask(() => flush())`.
*
* `onCleanup` is **not** allowed inside the callback — return a cleanup
* function instead. The returned cleanup runs on owner disposal.
*
* A cleanup return is only honored when `onSettled` is called from an **owned**
* scope (e.g. a component body). When it fires out of band from an *unowned*
* scope — an event handler, a tracked effect, or another `onSettled` — there is
* no owner lifecycle to bind a cleanup to; returning one is a dev-mode error
* (and is dropped in production). Use the post-settle/event-handler forms below
* for one-shot work, and keep setup-with-teardown in an owned scope.
*
* @example
* ```tsx
* // Component-level setup + teardown — replaces onMount + onCleanup.
* // Subscribe to an external source on mount, unsubscribe on dispose.
* function useViewportWidth() {
*   const [width, setWidth] = createSignal(window.innerWidth);
*   onSettled(() => {
*     const onResize = () => setWidth(window.innerWidth);
*     window.addEventListener("resize", onResize);
*     return () => window.removeEventListener("resize", onResize);
*   });
*   return width;
* }
* ```
*
* @example
* ```tsx
* // Post-settle "ready" hook — no cleanup needed.
* function Dashboard() {
*   const data = createMemo(async () => fetchData());
*
*   onSettled(() => {
*     analytics.track("dashboard.ready");
*   });
*
*   return <Loading fallback={<Spinner />}><pre>{data()}</pre></Loading>;
* }
* ```
*
* @example
* ```tsx
* // Event-handler — runs after the action settles.
* function SaveButton() {
*   const save = action(function* () {
*     yield api.save();
*   });
*
*   const handleClick = () => {
*     save();
*     onSettled(() => toast("Saved!"));
*   };
*
*   return <button onClick={handleClick}>Save</button>;
* }
* ```
*
* @param callback Function to run; may return a cleanup function that fires
*   on owner disposal
*/ function onSettled(e) {
	const t = getOwner();
	t && !(t.T & 16) ? trackedEffect(() => untrack(e), void 0) : globalQueue.enqueue(2, () => {
		e();
	});
}
var $PROXY = Symbol(0);
function boundaryComputed(e, t) {
	const r = computed(e, { lazy: true });
	ext(r).h = (e, t) => {
		const n = e !== void 0 ? e : r.S;
		const s = t !== void 0 ? t : r.o?._;
		r.S &= ~r.R;
		const i = r.C.notify(r, 3, n, s);
		const o = n & ~r.R & 3;
		if (o) {
			r.S &= ~o;
			if (r.o?._ === s && !(r.S & 3)) {
				if (r.o !== null) r.o._ = void 0;
			}
		}
		if (!i && n & 2) {
			haltReactivity(unwrapStatusError(s));
			throw s;
		}
	};
	r.R = t;
	r.T &= -33;
	recompute(r, true);
	return r;
}
function createBoundChildren(e, t, r, n) {
	const s = e.C;
	s.addChild(e.C = r);
	cleanup(() => s.removeChild(e.C));
	return runWithOwner(e, () => {
		const e = computed(t);
		return boundaryComputed(() => flatten(read(e)), n);
	});
}
var ON_INIT = Symbol();
var RevealControllerContext = /* @__PURE__ */ createContext(null);
var _revealUsed = false;
var CollectionQueue = class extends Queue {
	ee;
	v = /* @__PURE__ */ new Set();
	te;
	U = true;
	D = signal(false, {
		ownedWrite: true,
		H: true
	});
	_;
	P = signal(false, {
		ownedWrite: true,
		H: true
	});
	W;
	L = false;
	re;
	ne = ON_INIT;
	constructor(e) {
		super();
		this.ee = e;
	}
	run(e) {
		if (!e || read(this.D) && (!_revealUsed || read(this.P))) return;
		return super.run(e);
	}
	notify(e, t, r, n) {
		if (!(t & this.ee)) return super.notify(e, t, r, n);
		if (this.L && this.re) {
			const e = untrack(() => {
				try {
					return this.re();
				} catch {
					return ON_INIT;
				}
			});
			if (e !== this.ne) {
				this.ne = e;
				this.L = false;
				this.v.clear();
			}
		}
		if (this.ee & 1 && this.L) return super.notify(e, t, r, n);
		if (r & this.ee) {
			this.U = true;
			const t = n?.source || e.o?._?.source;
			if (t) {
				const e = this.v.size === 0;
				this.v.add(t);
				if (e) setSignal(this.D, true);
				if (this.ee & 2) setSignal(this._, unwrapStatusError(t.o?._));
			}
		}
		t &= ~this.ee;
		return t ? super.notify(e, t, r, n) : true;
	}
	se() {
		for (const e of this.v) if (e.ie & 64 || !e.o?.t && !(e.S & this.ee) && !(this.ee & 2 && e.S & 1)) this.v.delete(e);
		if (!this.v.size) {
			if (this.ee & 1 && this.U && !this.L && this.te) this.U = !!(this.te.S & this.ee);
			else this.U = false;
			if (!this.U) {
				setSignal(this.D, false);
				if (this.re) try {
					this.ne = untrack(() => this.re());
				} catch {}
			}
		}
		if (_revealUsed) this.W?.B();
	}
};
function createCollectionBoundary(e, t, r, n) {
	const s = createOwner();
	if (_revealUsed) setContext(RevealControllerContext, null, s);
	const i = new CollectionQueue(e);
	if (e === 2) i._ = signal(void 0, {
		ownedWrite: true,
		H: true
	});
	if (n) i.re = n;
	const o = i.te = createBoundChildren(s, t, i, e);
	untrack(() => {
		let t = false;
		try {
			read(o);
		} catch (e) {
			if (e instanceof NotReadyError) t = true;
			else throw e;
		}
		i.U = t || !!(o.S & e) || o.o?._ instanceof NotReadyError;
	});
	const l = _revealUsed && e === 1 ? getContext(RevealControllerContext) : null;
	if (l) {
		i.W = l;
		l.Z(i);
		cleanup(() => l.$(i));
	}
	return accessor(computed(() => {
		if (!read(i.D)) {
			const e = read(o);
			if (!untrack(() => read(i.D))) return i.L = true, e;
		}
		if (_revealUsed && read(i.P)) return void 0;
		return r(i);
	}, { H: true }));
}
/**
* Lower-level primitive that backs the `<Errored>` flow control. Catches
* thrown errors inside `fn` and invokes `fallback(error, reset)` instead.
* `error` is an accessor for the latest captured error; `reset()` recomputes
* the failing sources so the boundary can attempt to recover.
*
* App code should use `<Errored fallback={...}>` instead — reach for this only
* when authoring custom boundary components.
*
* @example
* ```tsx
* // Custom boundary that wraps the primitive and adds telemetry.
* function TracedErrored(props: { fallback: (e: () => unknown) => JSX.Element; children: JSX.Element }) {
*   return createErrorBoundary(
*     () => props.children,
*     (err, reset) => {
*       reportError(err());
*       return props.fallback(err);
*     }
*   ) as unknown as JSX.Element;
* }
* ```
*/ function createErrorBoundary$1(e, t) {
	return createCollectionBoundary(2, e, (e) => t(accessor(e._), () => {
		for (const t of e.v) if (t.oe !== void 0) recompute(t);
		schedule();
	}));
}
/**
* Resolves a children value to its renderable form: unwraps zero-arg functions
* (accessors), recursively flattens arrays, and optionally skips
* non-rendering values (`null`, `undefined`, `true`, `false`, `""`).
*
* Used internally by flow components and by the renderer to walk a children
* tree. App code rarely needs this directly — see `children()` in `solid-js`
* for the user-facing helper that memoizes the result.
*
* @param children value or array of values to flatten
* @param options
*   - `skipNonRendered` — drop values that won't render
*   - `doNotUnwrap` — leave function children as-is (caller will resolve)
*
* @example
* ```ts
* // Custom renderer walking a children tree manually. Most authors should
* // use `children()` from solid-js, which memoizes the resolved value.
* function renderChildren(value: unknown): unknown {
*   return flatten(value, { skipNonRendered: true });
* }
* ```
*/ function flatten(e, t) {
	if (typeof e === "function" && !e.length) {
		if (t?.doNotUnwrap) return e;
		do
			e = e();
		while (typeof e === "function" && !e.length);
	}
	if (t?.skipNonRendered && (e == null || e === true || e === false || e === "")) return;
	if (Array.isArray(e)) {
		let r = [];
		if (flattenArray(e, r, t)) return () => {
			let e = [];
			flattenArray(r, e, {
				...t,
				doNotUnwrap: false
			});
			return e;
		};
		return r;
	}
	return e;
}
function flattenArray(e, t = [], r) {
	let n = null;
	let s = false;
	for (let i = 0; i < e.length; i++) try {
		let n = e[i];
		if (typeof n === "function" && !n.length) {
			if (r?.doNotUnwrap) {
				t.push(n);
				s = true;
				continue;
			}
			do
				n = n();
			while (typeof n === "function" && !n.length);
		}
		if (Array.isArray(n)) s = flattenArray(n, t, r) || s;
		else if (r?.skipNonRendered && (n == null || n === true || n === false || n === "")) {} else t.push(n);
	} catch (e) {
		if (!(e instanceof NotReadyError)) throw e;
		n = e;
	}
	if (n) throw n;
	return s;
}
var $SOURCES = Symbol(0);
/** @internal The flattened sources behind a `merge()` PROXY, or undefined.
* Only the proxy form: its writes are no-ops, so the sources are the whole
* truth. merge()'s plain-object form also records `$SOURCES` (so nested
* merges flatten), but it is a real object callers may mutate afterwards
* (html's tagged templates assign props after spreading) — those own writes
* live on the object, not in the sources, so it must be read directly. */ function mergeSources(e) {
	return e != null && e[$PROXY] === e ? e[$SOURCES] : void 0;
}
var sharedConfig = {
	hydrating: false,
	registry: void 0,
	done: false
};
var createSignal = (...args) => {
	return createSignal$1(...args);
};
var createErrorBoundary = (...args) => createErrorBoundary$1(...args);
var createRenderEffect = (...args) => createRenderEffect$1(...args);
function createComponent(Comp, props, name) {
	return untrack(() => Comp(props || {}));
}
function Errored(props) {
	return createErrorBoundary(() => props.children, (err, reset) => {
		const f = props.fallback;
		return typeof f === "function" && f.length ? f(err, reset) : f;
	});
}
var DOMWithState = {
	INPUT: {
		value: 1,
		defaultValue: 2,
		checked: 1,
		defaultChecked: 2
	},
	SELECT: { value: 1 },
	OPTION: {
		value: 1,
		selected: 1,
		defaultSelected: 2
	},
	TEXTAREA: {
		value: 1,
		defaultValue: 2
	},
	VIDEO: {
		muted: 1,
		defaultMuted: 2
	},
	AUDIO: {
		muted: 1,
		defaultMuted: 2
	}
};
var ChildProperties = /*#__PURE__*/ new Set([
	"innerHTML",
	"textContent",
	"innerText",
	"children"
]);
var $$SLOT = /*#__PURE__*/ Symbol("slot");
var $$HOST = /*#__PURE__*/ Symbol("host");
var DelegatedEvents = /*#__PURE__*/ new Set([
	"beforeinput",
	"click",
	"dblclick",
	"contextmenu",
	"focusin",
	"focusout",
	"input",
	"keydown",
	"keyup",
	"mousedown",
	"mousemove",
	"mouseout",
	"mouseover",
	"mouseup",
	"pointerdown",
	"pointermove",
	"pointerout",
	"pointerover",
	"pointerup",
	"touchend",
	"touchmove",
	"touchstart"
]);
var Namespaces = {
	svg: "http://www.w3.org/2000/svg",
	mathml: "http://www.w3.org/1998/Math/MathML",
	xlink: "http://www.w3.org/1999/xlink",
	xml: "http://www.w3.org/XML/1998/namespace"
};
var transparentOptions = {
	transparent: true,
	sync: true
};
function effect(fn, effectFn, options) {
	createRenderEffect(fn, effectFn, options ? {
		sync: true,
		...options,
		transparent: !options.scope
	} : transparentOptions);
}
function reconcileArrays(parentNode, a, b, marker) {
	let bLength = b.length, aEnd = a.length, bEnd = bLength, aStart = 0, bStart = 0, tail = a[aEnd - 1], tailTag = tail[$$SLOT], after = tail.parentNode === parentNode && (!tailTag || tailTag === marker) ? tail.nextSibling : marker || null, map = null, anchor, anchorTag;
	const isLive = (n) => {
		if (!n) return false;
		const tag = n[$$SLOT];
		return n.parentNode === parentNode && (!tag || tag === marker);
	};
	while (aStart < aEnd || bStart < bEnd) {
		if (a[aStart] === b[bStart] && isLive(a[aStart])) {
			aStart++;
			bStart++;
			continue;
		}
		while (a[aEnd - 1] === b[bEnd - 1] && isLive(a[aEnd - 1])) {
			aEnd--;
			bEnd--;
		}
		if (aEnd === aStart) {
			let node;
			if (bEnd < bLength) {
				if (bStart) {
					const prev = b[bStart - 1];
					const prevTag = prev[$$SLOT];
					node = prev.parentNode === parentNode && (!prevTag || prevTag === marker) ? prev.nextSibling : after;
				} else node = b[bEnd - bStart];
			} else node = after;
			while (bStart < bEnd) {
				const n = b[bStart++];
				parentNode.insertBefore(n, node);
				if (marker) n[$$SLOT] = marker;
			}
		} else if (bEnd === bStart) while (aStart < aEnd) {
			const n = a[aStart++];
			if (!map || !map.has(n)) {
				const tag = n[$$SLOT];
				if (n.parentNode === parentNode && (!tag || tag === marker)) n.remove();
			}
		}
		else if ((anchor = a[aStart]) === b[bEnd - 1] && b[bStart] === a[aEnd - 1] && anchor.parentNode === parentNode && (!(anchorTag = anchor[$$SLOT]) || anchorTag === marker)) {
			if (marker) do {
				const n = a[--aEnd];
				parentNode.insertBefore(n, anchor);
				n[$$SLOT] = marker;
				bStart++;
				if (aStart >= aEnd - 1 || bStart >= bEnd) break;
			} while (a[aStart] === b[bEnd - 1] && b[bStart] === a[aEnd - 1]);
			else do {
				parentNode.insertBefore(a[--aEnd], anchor);
				bStart++;
				if (aStart >= aEnd - 1 || bStart >= bEnd) break;
			} while (a[aStart] === b[bEnd - 1] && b[bStart] === a[aEnd - 1]);
		} else {
			if (!map) {
				map = /* @__PURE__ */ new Map();
				let i = bStart;
				while (i < bEnd) map.set(b[i], i++);
			}
			const index = map.get(a[aStart]);
			if (index != null) {
				if (bStart < index && index < bEnd) {
					let i = aStart, sequence = 1, t;
					while (++i < aEnd && i < bEnd) {
						if ((t = map.get(a[i])) == null || t !== index + sequence) break;
						sequence++;
					}
					if (sequence > index - bStart) {
						const head = a[aStart];
						const headTag = head[$$SLOT];
						const node = head.parentNode === parentNode && (!headTag || headTag === marker) ? head : after;
						while (bStart < index) {
							const n = b[bStart++];
							parentNode.insertBefore(n, node);
							if (marker) n[$$SLOT] = marker;
						}
					} else {
						const oldNode = a[aStart++];
						const newNode = b[bStart++];
						const oldTag = oldNode[$$SLOT];
						if (oldNode.parentNode === parentNode && (!oldTag || oldTag === marker)) parentNode.replaceChild(newNode, oldNode);
						else parentNode.insertBefore(newNode, after);
						if (marker) newNode[$$SLOT] = marker;
					}
				} else aStart++;
			} else {
				const n = a[aStart++];
				const nTag = n[$$SLOT];
				if (n.parentNode === parentNode && (!nTag || nTag === marker)) n.remove();
			}
		}
	}
}
new RegExp(`(?:^|;\\s*)flash=([^;]+)`);
var $$EVENT_OWNER = "_$SOLID_EVENT_OWNER";
var $$EVENT_TUPLE = Symbol();
var hasOwn = Object.prototype.hasOwnProperty;
var INNER_OWNED = {};
var delegatedEvents = /* @__PURE__ */ new Set();
var delegatedContainers = /* @__PURE__ */ new Map();
function render(code, element, init, options = {}) {
	let disposer;
	registerDelegatedRoot(element);
	try {
		createRoot((dispose) => {
			disposer = dispose;
			if (element === document) {
				const tree = code();
				effect(() => flatten(tree), () => {});
			} else {
				const tree = code();
				insert(element, () => tree, element.firstChild ? null : void 0, init, {
					...options.insertOptions,
					schedule: true
				});
			}
		}, { id: options.renderId });
		flush();
	} catch (err) {
		if (disposer) disposer();
		unregisterDelegatedRoot(element);
		throw err;
	}
	return () => {
		disposer();
		unregisterDelegatedRoot(element);
		element.textContent = "";
	};
}
function create(html, bypassGuard, flag) {
	const t = document.createElement("template");
	t.innerHTML = html;
	return flag === 2 ? t.content.firstChild.firstChild : t.content.firstChild;
}
function template(html, flag) {
	let node;
	return flag === 1 ? (bypassGuard) => document.importNode(node || (node = create(html, bypassGuard, flag)), true) : (bypassGuard) => (node || (node = create(html, bypassGuard, flag))).cloneNode(true);
}
function delegateEvents(eventNames) {
	for (let i = 0, l = eventNames.length; i < l; i++) {
		const name = eventNames[i];
		if (!delegatedEvents.has(name)) {
			delegatedEvents.add(name);
			delegatedContainers.forEach((state, container) => attachDelegatedEvent(name, container, state));
		}
	}
}
function registerDelegatedRoot(root) {
	const state = registerDelegatedContainer(root, root);
	if (state) state.roots = (state.roots || 0) + 1;
}
function unregisterDelegatedRoot(root) {
	const state = delegatedContainers.get(root);
	if (state) state.roots > 1 ? state.roots-- : delete state.roots;
	unregisterDelegatedContainer(root, root);
}
function registerDelegatedContainer(container, owner = container) {
	if (!container || !owner) return;
	let state = delegatedContainers.get(container);
	if (!state) delegatedContainers.set(container, state = {
		owners: /* @__PURE__ */ new Map(),
		handlers: /* @__PURE__ */ new Map()
	});
	state.owners.set(owner, (state.owners.get(owner) || 0) + 1);
	delegatedEvents.forEach((name) => attachDelegatedEvent(name, container, state));
	return state;
}
function unregisterDelegatedContainer(container, owner = container) {
	const state = delegatedContainers.get(container);
	if (!state) return;
	const count = state.owners.get(owner);
	if (count > 1) state.owners.set(owner, count - 1);
	else state.owners.delete(owner);
	if (state.owners.size) return;
	state.handlers.forEach((handler, name) => container.removeEventListener(name, handler));
	delegatedContainers.delete(container);
}
function attachDelegatedEvent(name, container, state) {
	if (state.handlers.has(name)) return;
	const handler = (e) => eventHandler(e, container, state);
	state.handlers.set(name, handler);
	container.addEventListener(name, handler);
}
function findOwner(target, state) {
	let node = target;
	let distance = 0;
	while (node) {
		if (state.owners.has(node)) return {
			owner: node,
			distance
		};
		distance++;
		node = node._$host || node.parentNode || node.host;
	}
}
var claimHandlers = null;
function claimElement(node) {
	if (claimHandlers !== null) for (let i = 0; i < claimHandlers.length; i++) claimHandlers[i](node);
	return node;
}
function setAttribute(node, name, value) {
	if (isHydrating(node)) return;
	const selectMultiple = name === "multiple" && node.localName === "select";
	if (value == null || value === false) node.removeAttribute(name);
	else {
		node.setAttribute(name, value === true ? "" : value);
		if (selectMultiple && !node._$multiple) {
			const options = node.options;
			for (let i = 0; i < options.length; i++) if (options[i].defaultSelected) options[i].selected = true;
		}
	}
	if (selectMultiple) node._$multiple = true;
	if (claimHandlers !== null && (name === "href" || name === "action")) claimElement(node);
}
function setAttributeNS(node, namespace, name, value) {
	if (isHydrating(node)) return;
	if (value == null || value === false) node.removeAttributeNS(namespace, name.indexOf(":") > -1 ? name.split(":").pop() : name);
	else node.setAttributeNS(namespace, name, value === true ? "" : value);
}
function className(node, value, prev) {
	if (typeof value === "number") value = "" + value;
	if (typeof prev === "number") prev = "" + prev;
	if (isHydrating(node)) {
		node._$classes = value && typeof value === "object" ? classListToObject(value) : void 0;
		return;
	}
	if (value == null || value === false) {
		if (prev || node._$classes) {
			node.removeAttribute("class");
			node._$classes = void 0;
		}
		return;
	}
	if (typeof value === "string") {
		node._$classes = void 0;
		value !== prev && node.setAttribute("class", value);
		return;
	}
	let applied;
	if (typeof prev === "string") {
		applied = {};
		node.removeAttribute("class");
	} else applied = node._$classes || classListToObject(prev || {});
	value = classListToObject(value);
	const classKeys = Object.keys(value);
	const prevKeys = Object.keys(applied);
	let i, len;
	for (i = 0, len = prevKeys.length; i < len; i++) {
		const key = prevKeys[i];
		if (!key || key === "undefined" || value[key]) continue;
		node.classList.remove(key);
	}
	for (i = 0, len = classKeys.length; i < len; i++) {
		const key = classKeys[i], classValue = !!value[key];
		if (!key || key === "undefined" || applied[key] === classValue || !classValue) continue;
		node.classList.add(key);
	}
	node._$classes = value;
}
function addEvent(node, name, handler, delegate) {
	if (delegate) {
		const key = `$$${name}`;
		let data;
		if (Array.isArray(handler)) {
			data = handler[1];
			node[key] = handler[0];
		} else node[key] = handler;
		node[`${key}Data`] = data;
		return;
	}
	if (Array.isArray(handler)) {
		const handlerFn = handler[0];
		const listener = (e) => handlerFn.call(node, handler[1], e);
		listener[$$EVENT_TUPLE] = handler;
		node.addEventListener(name, listener);
		return listener;
	}
	node.addEventListener(name, handler, typeof handler !== "function" && handler);
	return handler;
}
function style(node, value, prev) {
	if (isHydrating(node)) return;
	if (!value) {
		if (prev || node._$styles) {
			setAttribute(node, "style");
			node._$styles = void 0;
		}
		return;
	}
	const nodeStyle = node.style;
	if (typeof value === "string") {
		node._$styles = void 0;
		return nodeStyle.cssText = value;
	}
	if (typeof prev === "string") {
		nodeStyle.cssText = "";
		prev = void 0;
	}
	let applied = node._$styles;
	if (!applied) applied = node._$styles = prev ? { ...prev } : {};
	let v, s;
	for (s in applied) if (!hasOwn.call(value, s) || value[s] == null) {
		nodeStyle.removeProperty(s);
		delete applied[s];
	}
	for (s in value) {
		if (!hasOwn.call(value, s)) continue;
		v = value[s];
		if (v != null && v !== applied[s]) {
			nodeStyle.setProperty(s, v);
			applied[s] = v;
		}
	}
}
function readShallow(value) {
	if (value === null || typeof value !== "object") return value;
	if (Array.isArray(value)) return value.map(readShallow);
	if (value[$PROXY] !== value) return value;
	const keys = ownKeys(value);
	const out = {};
	for (let i = 0; i < keys.length; i++) {
		const k = keys[i];
		if (typeof k === "string") out[k] = value[k];
	}
	return out;
}
function ownKeys(o) {
	return o[$PROXY] === o ? Reflect.ownKeys(o) : Object.keys(o);
}
function spread(node, props, skipChildren) {
	const prevProps = {};
	const get = () => (typeof props === "function" ? props() : props) ?? {};
	if (!skipChildren) insert(node, () => {
		const source = get();
		return hasOwn.call(source, "children") ? source.children : void 0;
	});
	effect(() => {
		const source = get();
		const r = hasOwn.call(source, "ref") && source.ref;
		(typeof r === "function" || Array.isArray(r)) && ref(() => r, node);
	}, () => {});
	effect(() => {
		const source = get();
		const newProps = {};
		const sources = mergeSources(source);
		if (sources !== void 0) for (let i = 0; i < sources.length; i++) {
			let s = sources[i];
			if (typeof s === "function") s = s();
			if (s != null) collectProps(newProps, s);
		}
		else collectProps(newProps, source);
		return newProps;
	}, (props) => assign(node, props, true, prevProps, true));
	return prevProps;
}
function collectProps(out, s) {
	const keys = ownKeys(s);
	for (let i = 0; i < keys.length; i++) {
		const prop = keys[i];
		if (typeof prop !== "string" || prop === "children" || prop === "ref") continue;
		const v = s[prop];
		out[prop] = prop === "style" || prop === "class" ? readShallow(v) : v;
	}
}
function applyRef(r, element) {
	Array.isArray(r) ? r.flat(Infinity).forEach((f) => f && f(element)) : r(element);
}
function ref(fn, element) {
	const resolved = untrack(fn);
	runWithOwner(null, () => applyRef(resolved, element));
}
var SCOPE_OPTIONS = { scope: true };
var hydrationRt = null;
function insert(parent, accessor, marker, initial, options) {
	const multi = marker !== void 0;
	const host = options && options.host;
	if (multi && !initial) initial = [];
	if (hydrationRt !== null) initial = hydrationRt.claimInitial(parent, multi, initial);
	if (typeof accessor !== "function") {
		accessor = normalize(accessor, initial, multi, true);
		if (typeof accessor !== "function") {
			insertExpression(parent, accessor, initial, marker);
			host && tagHost(accessor, host);
			return;
		}
	}
	if (multi && initial.length === 0) {
		const placeholder = document.createTextNode("");
		parent.insertBefore(placeholder, marker);
		initial = [placeholder];
	}
	let current = initial;
	effect((prev) => {
		if (hydrationRt !== null) current = hydrationRt.reclaimRegion(current, parent, marker);
		const value = normalize(accessor(), current, multi, true);
		if (typeof value !== "function") return value;
		effect(() => (hydrationRt !== null && (current = hydrationRt.reclaimRegion(current, parent, marker)), normalize(value, current, multi)), (inner) => {
			current = insertExpression(parent, inner, current, marker);
			host && tagHost(current, host);
		}, prev !== void 0 && !(options && options.schedule) ? {
			...options,
			schedule: true
		} : options);
		return INNER_OWNED;
	}, (value) => {
		if (value === INNER_OWNED) return;
		current = insertExpression(parent, value, current, marker);
		host && tagHost(current, host);
	}, accessor.$s ? options ? {
		...options,
		scope: true
	} : SCOPE_OPTIONS : options);
}
function assign(node, props, skipChildren, prevProps = {}, skipRef = false) {
	const nodeName = node.nodeName;
	props || (props = {});
	for (const prop in prevProps) if (!(prop in props)) {
		if (prop === "children") continue;
		prevProps[prop] = assignProp(node, prop, null, prevProps[prop], skipRef, nodeName);
	}
	for (const prop in props) {
		if (prop === "children") {
			if (!skipChildren) insertExpression(node, normalize(props.children, void 0, false));
			continue;
		}
		prevProps[prop] = assignProp(node, prop, props[prop], prevProps[prop], skipRef, nodeName);
	}
}
function isHydrating(node) {
	if (!sharedConfig.hydrating) return false;
	if (!node || node.isConnected) return true;
	const roots = sharedConfig.claimRoots;
	if (roots) {
		for (let i = 0; i < roots.length; i++) if (roots[i].contains(node)) return true;
	}
	return false;
}
function classListToObject(classList) {
	if (Array.isArray(classList)) {
		const result = {};
		flattenClassList(classList, result);
		classList = result;
	}
	if (classList && typeof classList === "object") {
		const result = {}, keys = Object.keys(classList);
		for (let i = 0, len = keys.length; i < len; i++) {
			const key = keys[i];
			if (!classList[key]) continue;
			const classNames = key.trim().split(/\s+/);
			for (let j = 0, nameLen = classNames.length; j < nameLen; j++) classNames[j] && (result[classNames[j]] = true);
		}
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
function assignProp(node, prop, value, prev, skipRef, nodeName) {
	if (prop === "style") return style(node, value, prev), value;
	if (prop === "class") return className(node, value, prev), value;
	if (value === prev && DOMWithState[nodeName]?.[prop] !== 1) return prev;
	if (prop === "ref") {
		if (!skipRef && value) ref(() => value, node);
		return value;
	}
	const hasNamespace = prop.indexOf(":") > -1;
	if (!hasNamespace && prop.slice(0, 2) === "on") {
		const name = prop.slice(2).toLowerCase();
		const delegate = DelegatedEvents.has(name);
		if (!delegate && prev) {
			if (Array.isArray(value) && typeof prev === "function" && prev[$$EVENT_TUPLE] === value) return prev;
			node.removeEventListener(name, prev, typeof prev !== "function" && prev);
		}
		if (delegate || value) {
			const attached = addEvent(node, name, value, delegate);
			delegate && delegateEvents([name]);
			if (!delegate) return attached;
		}
	} else if (hasNamespace && prop.slice(0, 5) === "prop:" || ChildProperties.has(prop) || DOMWithState[nodeName]?.[prop]) {
		if (hasNamespace) prop = prop.slice(5);
		else if (isHydrating(node)) return value;
		if (prop === "value" && nodeName === "SELECT") queueMicrotask(() => node.value = value) || (node.value = value);
		else if ((prop === "value" || prop === "defaultValue") && (nodeName === "INPUT" || nodeName === "TEXTAREA")) node[prop] = value ?? "";
		else node[prop] = value;
	} else {
		const ns = hasNamespace && Namespaces[prop.split(":")[0]];
		if (ns) setAttributeNS(node, ns, prop, value);
		else setAttribute(node, prop, value);
	}
	return value;
}
function eventHandler(e, container, state) {
	if (hydrationRt !== null && hydrationRt.dedupEvent(e)) return;
	const prev = e[$$EVENT_OWNER];
	let resumeNode;
	if (prev) {
		if (prev === true || prev === container || !container.contains(prev)) return;
		resumeNode = prev;
	}
	const owner = state && (state.owners.size === 1 && state.owners.has(container) ? container : findOwner(e.target, state)?.owner);
	if (state && !owner) return;
	if (owner && owner === resumeNode) return;
	e[$$EVENT_OWNER] = owner || true;
	let node = resumeNode || e.target;
	const key = `$$${e.type}`;
	const oriTarget = e.target;
	const boundary = owner || container || e.currentTarget;
	const retarget = (value) => Object.defineProperty(e, "target", {
		configurable: true,
		value
	});
	const handleNode = () => {
		let handler = node[key];
		if (handler === void 0 && node.hasAttribute && node.hasAttribute("_bnd")) {
			const seam = globalThis[Symbol.for("solid.bnd")];
			if (seam) handler = seam.resolve(node, e.type);
		}
		if (handler && !node.disabled) {
			const data = node[`${key}Data`];
			data !== void 0 ? handler.call(node, data, e) : typeof handler === "function" ? handler.call(node, e) : handler.handleEvent(e);
			if (e.cancelBubble) return;
		}
		node.host && typeof node.host !== "string" && !node.host._$host && node.contains(e.target) && retarget(node.host);
		return true;
	};
	const walkUpTree = () => {
		while (node && handleNode()) {
			if (node === boundary || node.parentNode === boundary) break;
			node = node._$host || node.parentNode || node.host;
		}
	};
	Object.defineProperty(e, "currentTarget", {
		configurable: true,
		get() {
			return node || boundary || document;
		}
	});
	if (resumeNode) {
		if (resumeNode === e.target) node = resumeNode._$host || resumeNode.parentNode || resumeNode.host;
		if (node && node !== boundary) walkUpTree();
	} else if (e.composedPath) {
		const path = e.composedPath();
		if (path.length) {
			retarget(path[0]);
			for (let i = 0; i < path.length; i++) {
				node = path[i];
				if (!handleNode()) break;
				if (node._$host) {
					node = node._$host;
					walkUpTree();
					break;
				}
				if (node === boundary || node.parentNode === boundary) break;
			}
		} else walkUpTree();
	} else walkUpTree();
	retarget(oriTarget);
}
function insertExpression(parent, value, current, marker) {
	if (hydrationRt !== null && isHydrating(parent)) {
		if (value && value !== current) {
			const arr = Array.isArray(value);
			for (const n of arr ? value : [value]) if (n && n.nodeType) {
				if (!isHydrating(n)) return current;
			} else if (arr && (typeof n === "string" || typeof n === "number")) return current;
		}
		return value;
	}
	if (value === current) return value;
	const t = typeof value, multi = marker !== void 0;
	if (t === "string" || t === "number") {
		const tc = typeof current;
		if (tc === "string" || tc === "number") parent.firstChild.data = value;
		else if (ownsAllChildren(parent, current)) parent.textContent = value;
		else {
			removeOwnedChildren(parent, current);
			parent.insertBefore(document.createTextNode(value), parent.firstChild);
		}
	} else if (value === void 0) cleanChildren(parent, current, marker);
	else if (value.nodeType) {
		if (Array.isArray(current)) cleanChildren(parent, current, multi ? marker : null, value);
		else if (current && current.nodeType) current.parentNode === parent ? parent.replaceChild(value, current) : parent.appendChild(value);
		else if (current && parent.firstChild) parent.replaceChild(value, parent.firstChild);
		else parent.appendChild(value);
		if (marker) value[$$SLOT] = marker;
	} else if (Array.isArray(value)) {
		const currentArray = current && Array.isArray(current);
		for (let i = 0, len = value.length; i < len; i++) {
			const item = value[i], t = typeof item;
			if (t === "string" || t === "number") {
				const prev = currentArray ? current[i] : void 0;
				if (prev && prev.nodeType === 3) {
					if (prev.data !== "" + item) prev.data = item;
					value[i] = prev;
				} else value[i] = document.createTextNode(item);
			}
		}
		if (value.length === 0) cleanChildren(parent, current, marker);
		else if (currentArray) {
			if (current.length === 0) appendNodes(parent, value, marker);
			else reconcileArrays(parent, current, value, marker);
		} else {
			current && cleanChildren(parent, current);
			appendNodes(parent, value);
		}
	}
	return value;
}
function normalize(value, current, multi, doNotUnwrap) {
	value = flatten(value, {
		skipNonRendered: true,
		doNotUnwrap
	});
	if (doNotUnwrap && typeof value === "function") return value;
	if (multi && !Array.isArray(value)) value = [value != null ? value : ""];
	if (sharedConfig.hydrating && Array.isArray(value)) for (let i = 0, len = value.length; i < len; i++) {
		const item = value[i], prev = current && current[i], t = typeof item;
		if ((t === "string" || t === "number") && prev && prev.nodeType === 3 && isHydrating(prev)) value[i] = prev;
	}
	return value;
}
function tagHost(value, host) {
	if (Array.isArray(value)) for (let i = 0, len = value.length; i < len; i++) tagHost(value[i], host);
	else if (value && value.nodeType && value[$$HOST] !== host) {
		value[$$HOST] = host;
		Object.defineProperty(value, "_$host", {
			get: host,
			configurable: true
		});
	}
}
function appendNodes(parent, array, marker = null) {
	for (let i = 0, len = array.length; i < len; i++) {
		const n = array[i];
		parent.insertBefore(n, marker);
		if (marker) n[$$SLOT] = marker;
	}
}
function ownsAllChildren(parent, current) {
	if (current == null) return true;
	if (Array.isArray(current)) return current.length ? parent.firstChild === current[0] && parent.lastChild === current[current.length - 1] : parent.firstChild === null;
	if (current === "") return parent.firstChild === null;
	if (current.nodeType) return parent.firstChild === current && parent.lastChild === current;
	const first = parent.firstChild;
	return first !== null && first.nodeType === 3 && parent.lastChild === first;
}
function removeOwnedChildren(parent, current) {
	if (Array.isArray(current)) for (let i = 0; i < current.length; i++) {
		const el = current[i];
		if (el.parentNode === parent) el.remove();
	}
	else if (current.nodeType) {
		if (current.parentNode === parent) current.remove();
	} else {
		const first = parent.firstChild;
		if (first && first.nodeType === 3) first.remove();
	}
}
function cleanChildren(parent, current, marker, replacement) {
	if (marker === void 0) {
		if (ownsAllChildren(parent, current)) return parent.textContent = "";
		return removeOwnedChildren(parent, current);
	}
	if (current.length) {
		let inserted = false;
		for (let i = current.length - 1; i >= 0; i--) {
			const el = current[i];
			if (replacement !== el) {
				const tag = el[$$SLOT];
				const owns = el.parentNode === parent && (!tag || tag === marker);
				if (replacement && !inserted && !i) owns ? parent.replaceChild(replacement, el) : parent.insertBefore(replacement, marker);
				else if (owns) el.remove();
			} else inserted = true;
		}
	} else if (replacement) parent.insertBefore(replacement, marker);
	if (replacement && marker) replacement[$$SLOT] = marker;
}
var _tmpl$$1 = /* @__PURE__ */ template(`<span style=font-size:1.5em;text-align:center;position:fixed;left:0;bottom:55%;width:100%>`);
function ErrorFallback(props) {
	console.error(props.error());
	var _el$ = _tmpl$$1();
	insert(_el$, "Error | Uncaught Client Exception");
	return _el$;
}
function DefaultErrorBoundary(props) {
	return createComponent(Errored, {
		fallback: (error) => createComponent(ErrorFallback, { error }),
		get children() {
			return props.children;
		}
	});
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
var _tmpl$ = /* @__PURE__ */ template(`<main><h1 class=x1ywg3o6>Hello Destack</h1><button>Count: `);
/** Shared styles compiled into the browser stylesheet. */
/** Theme shared by the server and browser renders. */
var theme = createTheme({
	appearance: "light",
	accent: "indigo"
});
/** Render the build fixture. */
function App() {
	const [count, setCount] = createSignal(0);
	onSettled(() => {
		document.addEventListener("click", focusHeading);
		return () => document.removeEventListener("click", focusHeading);
	});
	var _el$ = _tmpl$();
	var _el$3 = _el$.firstChild.nextSibling;
	_el$3.firstChild;
	spread(_el$, theme, true);
	_el$3.$$click = () => setCount(count() + 1);
	insert(_el$3, count, null);
	return _el$;
}
/** Focus the heading from a browser lifecycle callback. */
function focusHeading() {
	document.querySelector("h1")?.focus();
}
delegateEvents(["click"]);
render(() => createComponent(DefaultErrorBoundary, { get children() {
	return createComponent(App, {});
} }), document.body);

//# sourceMappingURL=virtual_solid-ssr-entry-client-6gsEq8xi.js.map