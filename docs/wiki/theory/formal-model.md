<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk> -->

# Formal Model of File System State

## Introduction

This document defines the formal model underlying JanusKey’s reversibility guarantees. We establish mathematical definitions for file system state, operations, and the conditions for reversibility.

## Definitions

### File System State

A file system state $S$ is a partial function:

$$S: \mathcal{P} \rightharpoonup \mathcal{F}$$

where:

- $\mathcal{P}$ is the set of all valid file paths

- $\mathcal{F}$ is the set of all file states

- $\rightharpoonup$ denotes a partial function (not all paths have files)

### File State

A file state $f \in \mathcal{F}$ is a tuple:

$$f = (c, m)$$

where:

- $c \in \mathcal{C}$ is the content (byte sequence)

- $m \in \mathcal{M}$ is the metadata

### Metadata

Metadata $m \in \mathcal{M}$ is a tuple:

$$m = (\pi, o, g, s, t, \ell)$$

where:

- $\pi \in \{0, \ldots, 0o7777\}$ is the permission mode

- $o \in \mathbb{N}$ is the owner uid

- $g \in \mathbb{N}$ is the group gid

- $s \in \mathbb{N}$ is the file size

- $t \in \mathbb{R}$ is the modification timestamp

- $\ell \in \mathcal{P} \cup \{\bot\}$ is the symlink target (or $\bot$ if not a symlink)

### Content Hash

The content hash function $H: \mathcal{C} \to \mathcal{H}$ satisfies:

$$\forall c_1, c_2 \in \mathcal{C}: H(c_1) = H(c_2) \Rightarrow c_1 = c_2 \text{ (with overwhelming probability)}$$

JanusKey uses SHA256 for $H$.

## Operations

### Operation Definition

An operation $O$ is a function:

$$O: \mathcal{S} \to \mathcal{S}$$

that transforms one file system state to another.

### Reversible Operation

An operation $O$ is **reversible** if there exists an inverse operation $O^{-1}$ such that:

$$\forall S \in \mathcal{S}: O^{-1}(O(S)) = S$$

### Operation Catalog

#### Delete

$$\text{DELETE}_p: S \mapsto S' \text{ where } S'(q) = \begin{cases} \bot & \text{if } q = p \\ S(q) & \text{otherwise} \end{cases}$$

**Precondition**: $S(p) \neq \bot$

**Inverse**: $\text{CREATE}_p$ with stored content and metadata

#### Create

$$\text{CREATE}_{p,c,m}: S \mapsto S' \text{ where } S'(q) = \begin{cases} (c, m) & \text{if } q = p \\ S(q) & \text{otherwise} \end{cases}$$

**Precondition**: $S(p) = \bot$

**Inverse**: $\text{DELETE}_p$

#### Modify

$$\text{MODIFY}_{p,c'}: S \mapsto S' \text{ where } S'(q) = \begin{cases} (c', m) & \text{if } q = p \land S(p) = (c, m) \\ S(q) & \text{otherwise} \end{cases}$$

**Precondition**: $S(p) \neq \bot$

**Inverse**: $\text{MODIFY}_{p,c}$ with original content $c$

#### Move

$$\text{MOVE}_{p,q}: S \mapsto S' \text{ where } S'(r) = \begin{cases} \bot & \text{if } r = p \\ S(p) & \text{if } r = q \\ S(r) & \text{otherwise} \end{cases}$$

**Precondition**: $S(p) \neq \bot \land S(q) = \bot$

**Inverse**: $\text{MOVE}_{q,p}$

#### Copy

$$\text{COPY}_{p,q}: S \mapsto S' \text{ where } S'(r) = \begin{cases} S(p) & \text{if } r = q \\ S(r) & \text{otherwise} \end{cases}$$

**Precondition**: $S(p) \neq \bot \land S(q) = \bot$

**Inverse**: $\text{DELETE}_q$

#### Append

$$\text{APPEND}_{p,c'}: S \mapsto S' \text{ where } S'(q) = \begin{cases} (c \cdot c', m') & \text{if } q = p \land S(p) = (c, m) \\ S(q) & \text{otherwise} \end{cases}$$

where $c \cdot c'$ denotes concatenation and $m'$ updates size.

**Precondition**: $S(p) \neq \bot$

**Inverse**: $\text{TRUNCATE}_{p,|c|}$ to original size

#### Truncate

$$\text{TRUNCATE}_{p,n}: S \mapsto S' \text{ where } S'(q) = \begin{cases} (c[0:n], m') & \text{if } q = p \land S(p) = (c, m) \\ S(q) & \text{otherwise} \end{cases}$$

**Precondition**: $S(p) \neq \bot$

**Inverse**: $\text{MODIFY}_{p,c}$ with original content

## Theorems

### Theorem 1: Individual Reversibility

Every JanusKey operation is reversible.

#### Proof

We show for each operation type:

| Operation | Inverse                      | Reversibility Condition                    |
|-----------|------------------------------|--------------------------------------------|
| DELETE    | CREATE with stored (c,m)     | Content and metadata stored before delete  |
| CREATE    | DELETE                       | No information loss (creation is addition) |
| MODIFY    | MODIFY with original c       | Original content stored                    |
| MOVE      | MOVE in reverse direction    | Source and destination paths stored        |
| COPY      | DELETE of copy               | Destination path stored                    |
| APPEND    | TRUNCATE to original size    | Original size stored in metadata           |
| TRUNCATE  | MODIFY with original content | Original content stored                    |

All inverse information is captured by `OperationMetadata`. ∎

### Theorem 2: Composition Reversibility

If $O_1, O_2$ are reversible operations, then $O_2 \circ O_1$ is reversible with inverse $O_1^{-1} \circ O_2^{-1}$.

#### Proof

Let $S$ be the initial state.

$$(O_1^{-1} \circ O_2^{-1})(O_2 \circ O_1)(S) = O_1^{-1}(O_2^{-1}(O_2(O_1(S))))$$

By reversibility of $O_2$:

$$= O_1^{-1}(O_1(S))$$

By reversibility of $O_1$:

$$= S$$

∎

### Theorem 3: Sequential Reversibility (Theorem 3.4)

For any sequence of reversible operations $\langle O_1, O_2, \ldots, O_n \rangle$:

$$\langle O_n^{-1}, O_{n-1}^{-1}, \ldots, O_1^{-1} \rangle \text{ reverses } \langle O_1, O_2, \ldots, O_n \rangle$$

#### Proof

By induction on $n$, using Theorem 2. ∎

### Theorem 4: Transaction Atomicity

A transaction $T = \langle O_1, \ldots, O_n \rangle$ with rollback satisfies:

Either:

1.  **Commit**: All operations applied, all reversible

2.  **Rollback**: State equals initial state

#### Proof

Rollback executes $\langle O_n^{-1}, \ldots, O_1^{-1} \rangle$. By Theorem 3, this restores initial state. ∎

## Content-Addressed Storage Properties

### Definition: Content Store

A content store $\Sigma$ is a partial function:

$$\Sigma: \mathcal{H} \rightharpoonup \mathcal{C}$$

with operations:

- $\text{store}(c) = h$ where $h = H(c)$ and $\Sigma(h) := c$

- $\text{retrieve}(h) = \Sigma(h)$

### Theorem 5: Content Integrity

$$\forall c: \text{retrieve}(\text{store}(c)) = c$$

#### Proof

By definition of store and retrieve, and the collision resistance of $H$. ∎

### Theorem 6: Deduplication Correctness

$$\forall c_1, c_2: H(c_1) = H(c_2) \Rightarrow \text{store}(c_1) = \text{store}(c_2)$$

And both can be retrieved correctly.

#### Proof

If $H(c_1) = H(c_2)$, then by collision resistance, $c_1 = c_2$. Storage of either produces the same $h$, and retrieval returns the identical content. ∎

## Invariants

### Invariant 1: Metadata Completeness

For every operation $O$ recorded in the metadata store:

$$\text{sufficient}(\text{metadata}(O), O^{-1})$$

The metadata contains all information needed to compute the inverse.

### Invariant 2: Content Availability

For every content hash $h$ referenced in operation metadata:

$$\Sigma(h) \neq \bot$$

Referenced content is always available in the content store.

### Invariant 3: Operation Ordering

The metadata store maintains temporal ordering:

$$\forall O_i, O_j: i < j \Rightarrow \text{timestamp}(O_i) \leq \text{timestamp}(O_j)$$

## Further Reading

- [Extended Reversibility Theory](reversibility-theory.md)

- [RMR Primitive Specification](rmr-primitive.md)

- [Complete Formal Proofs](../formal-proofs/index.md)
