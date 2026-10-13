-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection: closing a library handle twice. `closeOnce` takes the
-- handle at multiplicity 1, so two closes of one linear handle are refused
-- with a use count of 2. The two closes sit in a linear pair (`#`): plain IO
-- do-notation would not test this, because IO's bind is not linear and even
-- a single close there is refused ("non-linear context"). A single close,
-- `closeOnce h` or `closeOnce h # pure ()`, typechecks. If this file ever
-- typechecks, the destroy-at-most-once obligation has been lost.
-- expect: There are 2 uses of linear name h
module JanusKey.ABI.Reject.DoubleClose

import JanusKey.ABI.Foreign

%default total

twice : (1 h : LibHandle Open) -> LPair (IO (LibHandle Closed)) (IO (LibHandle Closed))
twice h = closeOnce h # closeOnce h
