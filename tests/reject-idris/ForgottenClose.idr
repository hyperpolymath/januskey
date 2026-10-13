-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection: a linear handle that is never closed. The parameter
-- has multiplicity 1 and is not used, which Idris2 refuses.
-- expect: There are 0 uses of linear name h
module JanusKey.ABI.Reject.ForgottenClose

import JanusKey.ABI.Foreign

%default total

leak : (1 h : LibHandle Open) -> IO ()
leak h = pure ()
