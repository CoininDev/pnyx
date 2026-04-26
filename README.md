# Pnyx

> Decentralized voting infrastructure for libertarian municipalism — built in Rust.

Pnyx is a Byzantine fault-tolerant blockchain designed for federated direct democracy. It draws from the tradition of the [Pnyx](https://en.wikipedia.org/wiki/Pnyx) — the hill in Athens where citizens gathered to vote — and applies it to a cryptographically scoped, prunable, multi-commune network.

---

## Core concepts

### Scoped Merkle Patricia Trie (MPT)

All state lives in a single database, cryptographically partitioned into independent scopes. Each scope has its own `root_hash`, allowing nodes to carry only the data they need.

```
/conf/laws/042          → confederal scope   (all nodes carry this)
/commune/cypherpunx/laws/042  → commune scope (only local nodes required)
```

The confederal MPT stores the `root_hash` of every registered commune, enabling the entire network to validate any commune's state without holding its full history.

### Two-layer architecture

| Layer | Scope prefix | Who carries it |
|---|---|---|
| Confederation | `/conf/` | Every node |
| Commune | `/commune/<name>/` | Commune nodes + storage nodes |

### Communes

A commune is a self-governing unit with its own membership registry, laws, contracts, and nodes. Each commune's MPT subtree contains:

```
/commune/<name>/members/*     — registered keypairs
/commune/<name>/nodes/*       — registered node identifiers
/commune/<name>/laws/*        — permanent votes (reserved in MPT)
/commune/<name>/contracts/*   — smx smart contracts
/commune/<name>/notes/*       — ephemeral data (prunable)
```

---

## Node types

**Commune node (light)** — carries the confederal scope and its own commune scope. Stores only the N most recent blocks. Lowest hardware barrier; the default for commune members.

**Storage node** — carries full history for one or more communes. More expensive to operate; not required for participation.

**Read-only node** — can read any commune's state, cannot write. Useful for observers, auditors, and indexers.

Any node can *read* from any commune scope. Only nodes registered in a commune can *write* to it.

---

## Node identity and signing

Each node's signing keypair is derived from its maintainer's personal key and the node's ID:

```
node_key = HKDF(maintainer_privkey, node_id)
```

A node must be registered in the commune's `/nodes/*` subtree to have its blocks recognized. Unregistered nodes' blocks are ignored by the network. If a maintainer is banned, all their nodes are invalidated atomically alongside the ban transaction.

---

## Consensus

Pnyx uses **Tendermint BFT** for consensus within each commune's validator set. Block proposals are signed by node keys. The confederal chain runs its own Tendermint instance over confederal validators.

---

## Pruning

### Ephemeral vs permanent votes

| Type | Storage | Pruning |
|---|---|---|
| Permanent | Reserved in MPT | Never pruned |
| Ephemeral | On-chain only | Light nodes keep last N blocks |

Permanent votes (laws, membership changes, contract deployments) are committed to the MPT and survive pruning. Ephemeral votes live only on the chain and are dropped by light nodes once outside the retention window.

### Scope pruning

Light nodes hold the full confederal scope but only their local commune scope. They do not need to sync other communes' chains — only their `root_hash` entries in the confederal MPT.

---

## Smart contracts — smx

Pnyx includes an embedded functional language called **smx** for writing on-chain contracts. Contracts are registered at:

```
/commune/<name>/contracts/<contract_name>
```

### Domain isolation

Every contract declares a **domain** — the MPT subtree it is permitted to write to. A contract outside its domain cannot affect other subtrees, even if buggy or malicious.

```
create_law    → domain: /commune/<name>/laws/*
create_note   → domain: /commune/<name>/notes/*
create_commune → domain: /conf/communes/*  (confederal scope only)
```

Cross-domain writes are rejected at the runtime level. A broken contract can only corrupt its own domain.

### Scope semantics

Some contracts are only valid in certain scopes. `create_commune` is a confederal contract and has no meaning inside a commune scope. Commune contracts may not make sense in other communes. The smx runtime enforces this at dispatch time.

---

## Commune lifecycle

### Founding

1. A group of people decide to found a commune.
2. They submit a `create_commune` transaction to the confederal chain, establishing the commune's name and their keypairs as founding members.
3. The commune's initial `root_hash` is registered in the confederal MPT.
4. Founding members become the first validated membership set.

### Membership

- The community votes to recognize a keypair as a new member.
- Members can be banned by community vote.
- Each member may control zero or more nodes; each node has exactly one maintainer.

### Regular schism (fork)

A group that wishes to split from a commune may submit a `fork_commune` transaction, choosing a new name and carrying a linked history reference. Both communes are registered as legitimate in the confederal MPT. This is the sanctioned path for ideological splits.

### Irregular schism (conflict resolution)

If two groups claim the same commune name with diverging chains, the confederation detects the conflict when two distinct `root_hash` values are submitted for the same commune in the same confederal slot.

Resolution proceeds as follows:

1. **Detection** — a confederal member submits a `report_schism` transaction with Merkle proofs for both competing chains.
2. **Evidence period** — both groups publish on-chain proofs (signature continuity, membership census, node liveness).
3. **Confederal vote** — registered confederal members vote to elect one `root_hash` as canonical.
4. **Resolution** — the elected chain is registered; the other's blocks are no longer recognized. The losing group may re-register under a new name via `fork_commune`.

If quorum is not reached within the voting window, the `root_hash` registered earliest in the confederal chain prevails.

---

## Path resolution

The MPT resolves scopes automatically from path prefixes. A query for `/commune/cypherpunx/laws/042` identifies `/commune/cypherpunx` as a scoped subtree with its own `root_hash`. A query for `/conf/laws/042` resolves against the confederal root. This means proof generation, verification, and pruning all operate per-scope without cross-contamination.

---

## Building

```bash
git clone https://github.com/coinindev/pnyx
cd pnyx
cargo build --release
```

---

## License

[TBD]