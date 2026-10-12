-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection: closing a library handle twice. `closeOnce` takes the
-- handle at multiplicity 1, so the second use is a type error. If this file
-- ever typechecks, the destroy-at-most-once obligation has been lost.
module JanusKey.ABI.Reject.DoubleClose

import JanusKey.ABI.Foreign

%default total

twice : (1 h : LibHandle Open) -> IO (LibHandle Closed)
twice h = do
  _ <- closeOnce h
  closeOnce h
