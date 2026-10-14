<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk> -->

# Theoretical Foundations

## Overview

JanusKey’s design is grounded in rigorous theoretical foundations that provide mathematical guarantees for reversibility. This section covers the core concepts that make JanusKey’s guarantees possible.

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

| Aspect       | Traditional                 | MPR                          |
|--------------|-----------------------------|------------------------------|
| Data safety  | Achieved through backups    | Impossible to lose by design |
| Verification | Post-hoc testing            | Compile-time guarantees      |
| Trust model  | Trust operational processes | Trust mathematical proofs    |
| Failure mode | Graceful degradation        | No degradation possible      |

## The MAA Framework

The **Mutually Assured Accountability (MAA) Framework** balances two opposing requirements:

1.  **Auditability**: The ability to track, verify, and replay all state changes

2.  **User Autonomy**: The right to permanently erase data (GDPR Article 17)

These are implemented through dual primitives:

- **RMR (Reversible Transaction)**: Guarantees perfect state reversal

- **RMO (Obliterative Wipe)**: Guarantees permanent, verifiable data erasure

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
