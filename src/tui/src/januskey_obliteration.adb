-- SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
-- SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
--
-- JanusKey SPARK Obliteration: Implementation
--
-- NOTE: These implementations model the behavior for SPARK verification.
-- Actual secure overwrite would call system-level primitives.

pragma SPARK_Mode (On);

package body JanusKey_Obliteration is

   ---------------------------------------------------------------------------
   -- Helper: Remove content from store
   ---------------------------------------------------------------------------

   procedure Remove_From_Store
     (Hash    : in     Content_Hash;
      Success :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => Content_Store),
     Pre  => Content_Exists (Hash),
     Post => (if Success then not Content_Exists (Hash))
   is
   begin
      for I in Content_Index loop
         if Content_Store (I).Hash = Hash and Content_Store (I).Stored then
            Content_Store (I).Stored := False;
            Success := True;
            return;
         end if;
      end loop;
      Success := False;
   end Remove_From_Store;

   ---------------------------------------------------------------------------
   -- Helper: Generate commitment hash
   -- In real implementation: H(content_hash || nonce || timestamp)
   ---------------------------------------------------------------------------

   function Generate_Commitment
     (Hash      : Content_Hash;
      Nonce     : Content_Hash;
      Timestamp : Natural) return Content_Hash
   with
     SPARK_Mode,
     Post => Is_Valid_Hash (Generate_Commitment'Result)
   is
      Result : Content_Hash := Null_Hash;
   begin
      -- Simplified: XOR hash with nonce (real impl uses SHA256)
      for I in Hash_Index loop
         if Hash (I) in '0' .. '9' then
            Result (I) := Hash (I);
         else
            Result (I) := Nonce (I);
         end if;
      end loop;
      return Result;
   end Generate_Commitment;

   ---------------------------------------------------------------------------
   -- Obliterate: Core RMO implementation
   ---------------------------------------------------------------------------

   procedure Obliterate
     (Hash    : in     Content_Hash;
      Reason  : in     String;
      Proof   :    out Obliteration_Proof;
      Success :    out Boolean)
   is
      pragma Unreferenced (Reason);
      Remove_OK : Boolean;
      Current_Time : constant Natural := 0;  -- Would be system time
      Nonce : constant Content_Hash := Null_Hash;  -- Would be random
   begin
      -- Step 1: Perform secure overwrite (modeled)
      -- In real implementation: 3+ passes of overwrite patterns

      -- Step 2: Remove from content store
      Remove_From_Store (Hash, Remove_OK);

      if Remove_OK then
         -- Step 3: Generate obliteration proof
         Proof := (Content_Hash     => Hash,
                   Timestamp        => Current_Time,
                   Nonce            => Nonce,
                   Commitment       => Generate_Commitment (Hash, Nonce, Current_Time),
                   Overwrite_Passes => Min_Overwrite_Passes,
                   Storage_Cleared  => True);

         -- Step 4: Record in obliteration log
         Obliteration_Next := Obliteration_Next + 1;
         Obliterations (Obliteration_Next) :=
           (Hash      => Hash,
            Proof     => Proof,
            Timestamp => Current_Time);

         Success := True;
      else
         Proof := (Content_Hash     => Null_Hash,
                   Timestamp        => 0,
                   Nonce            => Null_Hash,
                   Commitment       => Null_Hash,
                   Overwrite_Passes => 0,
                   Storage_Cleared  => False);
         Success := False;
      end if;
   end Obliterate;

   ---------------------------------------------------------------------------
   -- Verify_Obliteration_Proof
   ---------------------------------------------------------------------------

   function Verify_Obliteration_Proof
     (Proof : Obliteration_Proof) return Boolean
   is
   begin
      return Is_Valid_Obliteration (Proof);
   end Verify_Obliteration_Proof;

   ---------------------------------------------------------------------------
   -- Get_Obliteration_Count
   ---------------------------------------------------------------------------

   function Get_Obliteration_Count return Obliteration_Count
   is
   begin
      return Obliteration_Next;
   end Get_Obliteration_Count;

   ---------------------------------------------------------------------------
   -- Is_Content_Obliterated
   ---------------------------------------------------------------------------

   function Is_Content_Obliterated (Hash : Content_Hash) return Boolean
   is
   begin
      for I in 1 .. Obliteration_Next loop
         if Obliterations (I).Hash = Hash then
            return True;
         end if;
         pragma Loop_Invariant
           (for all J in 1 .. I => Obliterations (J).Hash /= Hash or J = I);
      end loop;
      return False;
   end Is_Content_Obliterated;

   ---------------------------------------------------------------------------
   -- Process_Erasure_Request (GDPR Article 17)
   ---------------------------------------------------------------------------

   procedure Process_Erasure_Request
     (Request : in     Erasure_Request;
      Proof   :    out Obliteration_Proof;
      Success :    out Boolean)
   is
   begin
      -- Delegate to core obliterate with GDPR context
      Obliterate (Hash    => Request.Content_Hash,
                  Reason  => "GDPR Article 17 Erasure Request",
                  Proof   => Proof,
                  Success => Success);
   end Process_Erasure_Request;

   ---------------------------------------------------------------------------
   -- Obliterate_Batch
   ---------------------------------------------------------------------------

   procedure Obliterate_Batch
     (Hashes       : in     Hash_Array;
      Reason       : in     String;
      Success_Count:    out Natural;
      All_Success  :    out Boolean)
   is
      Proof : Obliteration_Proof;
      OK    : Boolean;
   begin
      Success_Count := 0;
      All_Success := True;

      for I in Hashes'Range loop
         if Content_Exists (Hashes (I)) and not Was_Obliterated (Hashes (I)) then
            Obliterate (Hashes (I), Reason, Proof, OK);
            if OK then
               Success_Count := Success_Count + 1;
            else
               All_Success := False;
            end if;
         else
            All_Success := False;
         end if;

         pragma Loop_Invariant (Success_Count <= I - Hashes'First + 1);
         pragma Loop_Invariant
           (if All_Success then Success_Count = I - Hashes'First + 1);
      end loop;
   end Obliterate_Batch;

end JanusKey_Obliteration;
