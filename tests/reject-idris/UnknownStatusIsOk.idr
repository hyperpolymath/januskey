-- SPDX-License-Identifier: MPL-2.0
-- SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
-- Expected rejection: decoding a code the header does not define as success.
-- statusFromCode 12 is Nothing, so this equality cannot hold.
module JanusKey.ABI.Reject.UnknownStatusIsOk

import JanusKey.ABI.Foreign

%default total

twelveIsOk : statusFromCode 12 = Just Ok
twelveIsOk = Refl
