<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk> -->

# Theoretical Foundations

## Overview

JanusKey's design is stated over a formal model — file-system states, operations
and their inverses — and part of that model is checked by machine. The pages here
describe the intended theory and, in each case, how much of it is actually
discharged: three transaction lemmas are proved in Agda, the ABI lemmas are proved
in Idris2, and the claims that would matter most (that every operation is
reversible, that the model corresponds to the Rust) are OPEN and ASSUMED
respectively. `PROOF-NEEDS.adoc` is the authority.

### Contents

1.  [Maximal Principle Reduction (MPR)](mpr.md)

2.  [Mutually Assured Accountability (MAA) Framework](maa-framework.md)

3.  [RMR: Reversible Transaction Primitive](rmr-primitive.md)

4.  [RMO: Obliterative Wipe Primitive](rmo-primitive.md)

5.  [Formal Model of File System State](formal-model.md)

6.  [Reversibility Theory](reversibility-theory.md)

## Core Axiom: Maximal Principle Reduction

The foundational principle underlying JanusKey is **Maximal Principle Reduction (MPR)**:

> Security and correctness should be achieved by construction, not verification. The system should be designed such that vulnerabilities are structurally impossible, not merely unlikely or detectable.

### Traditional vs MPR Approach

| Aspect       | Traditional                 | MPR (as intended)                    |
|--------------|-----------------------------|--------------------------------------|
| Data safety  | Achieved through backups    | No code path destroys the inverse    |
| Verification | Post-hoc testing            | Design review; proofs where they exist |
| Trust model  | Trust operational processes | Trust the record, and the proof where one is discharged |
| Failure mode | Graceful degradation        | Not applicable — this is a CLI, and it can lose the store |

Read MPR as a design philosophy, not a method with tooling: the repository says so
itself (`README.adoc` §Known scope boundaries) and no proof in this tree is linked
to the Rust by machine (ledger row G-2 is ASSUMED).

## The MAA Framework

The **Mutually Assured Accountability (MAA) Framework** balances two opposing requirements:

1.  **Auditability**: The ability to track, verify, and replay all state changes

2.  **User Autonomy**: The right to permanently erase data (GDPR Article 17)

These are implemented through dual primitives:

- **RMR (Reversible Transaction)**: Records each change's inverse; undo restores
  the prior state up to a stated equality. This is tested for single steps. For
  a transaction model it is proved under a premise: every step executed and each
  step's undo restores the state that step started from (that premise is the
  single-step property, tested, not proved). The right-inverse direction and
  "every operation is reversible" are OPEN (see [MAA Property 1](maa-framework.md))

- **RMO (Obliterative Wipe)**: Best-effort overwrite and removal. When a
  JanusKey store exists, each removal is written to an unkeyed JSON ledger; a
  writer can alter it without detection, and with no store no record is kept.
  The ledger is written after the bytes are removed, so a failed ledger write
  can leave a removal unrecorded, and a re-run skips the blob that is already
  gone without recording it. Unrecoverability is OPEN (E-1) and physical
  erasure is ASSUMED (X-1)

<!-- -->

                        +----------------+
                        |  MAA Framework |
                        +-------+--------+
                                |
                +---------------+---------------+
                |                               |
        +-------v-------+               +-------v-------+
        |     RMR       |               |     RMO       |
        | Reversibility |               | Obliteration  |
        +-------+-------+               +-------+-------+
                |                               |
        +-------v-------+               +-------v-------+
        | Auditability  |               |   Autonomy    |
        | Accountability|               |   Privacy     |
        +---------------+               +---------------+

## Mathematical Foundation

### File System State

A file system state $S$ is defined as a total function:

$$S: \text{Path} \to \text{FileState}$$

where:

$$\text{FileState} = \text{Content} \times \text{Metadata} \cup \{\bot\}$$

The symbol $\bot$ represents a non-existent file.

### Operations as Functions

Each operation $O$ is a function:

$$O: S \to S$$

For JanusKey operations, we require the existence of an inverse:

$$\forall O \in \text{Operations}: \exists O^{-1}: O^{-1}(O(S)) = S$$

## Historical Context

JanusKey builds on foundational work in:

- **Algorithmic Reversibility** (Bennett, 1973): Theoretical foundation for reversible computing

- **Landauer’s Principle**: Thermodynamic limits of information erasure

- **ACID Properties**: Transaction guarantees in database systems

- **Content-Addressable Storage**: Git’s object model for integrity

## Next Steps

- [Deep dive into Maximal Principle Reduction](mpr.md)

- [Formal model definitions and proofs](formal-model.md)

- [See how theory maps to implementation](../architecture/index.md)
