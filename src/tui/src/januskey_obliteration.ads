-- SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
-- SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
--
-- JanusKey SPARK Obliteration: Formally verified RMO primitive
--
-- THEOREM (Formal Obliteration):
--   After obliterate(hash):
--   1. Content is cryptographically unrecoverable
--   2. A verifiable proof of non-existence is generated
--   3. The fact of obliteration is logged (GDPR Article 17)
--
-- This package provides SPARK contracts for the RMO primitive.

pragma SPARK_Mode (On);

with JanusKey_Types; use JanusKey_Types;

package JanusKey_Obliteration is

   ---------------------------------------------------------------------------
   -- Obliteration State (Ghost variables for specification)
   ---------------------------------------------------------------------------

   type Obliteration_Record is record
      Hash      : Content_Hash;
      Proof     : Obliteration_Proof;
      Timestamp : Natural;
   end record;

   Max_Obliterations : constant := 10_000;
   subtype Obliteration_Count is Natural range 0 .. Max_Obliterations;
   subtype Obliteration_Index is Positive range 1 .. Max_Obliterations;

   type Obliteration_Log is array (Obliteration_Index) of Obliteration_Record;

   Obliterations     : Obliteration_Log with Ghost;
   Obliteration_Next : Obliteration_Count := 0 with Ghost;

   -- Ghost state for content store
   Content_Store : Content_Store_State with Ghost;

   ---------------------------------------------------------------------------
   -- Predicates
   ---------------------------------------------------------------------------

   function Content_Exists (Hash : Content_Hash) return Boolean is
     (Content_Available (Content_Store, Hash))
   with Ghost;

   function Was_Obliterated (Hash : Content_Hash) return Boolean is
     (for some I in 1 .. Obliteration_Next =>
        Obliterations (I).Hash = Hash)
   with Ghost;

   function Has_Valid_Proof (Hash : Content_Hash) return Boolean is
     (for some I in 1 .. Obliteration_Next =>
        Obliterations (I).Hash = Hash and
        Is_Valid_Obliteration (Obliterations (I).Proof))
   with Ghost;

   ---------------------------------------------------------------------------
   -- RMO Primitive: Obliterate Content
   ---------------------------------------------------------------------------

   -- Minimum secure overwrite passes (DoD 5220.22-M standard)
   Min_Overwrite_Passes : constant := 3;

   procedure Obliterate
     (Hash    : in     Content_Hash;
      Reason  : in     String;
      Proof   :    out Obliteration_Proof;
      Success :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => (Content_Store, Obliterations, Obliteration_Next)),
     Pre    => Is_Valid_Hash (Hash) and
               Content_Exists (Hash) and
               not Was_Obliterated (Hash) and
               Obliteration_Next < Max_Obliterations,
     Post   => (if Success then
                  -- THEOREM 1: Content is no longer available
                  not Content_Exists (Hash) and

                  -- THEOREM 2: Obliteration is recorded
                  Was_Obliterated (Hash) and

                  -- THEOREM 3: Valid proof exists
                  Has_Valid_Proof (Hash) and
                  Is_Valid_Obliteration (Proof) and

                  -- THEOREM 4: Proof references correct content
                  Proof.Content_Hash = Hash and

                  -- THEOREM 5: Secure overwrite performed
                  Proof.Overwrite_Passes >= Min_Overwrite_Passes and
                  Proof.Storage_Cleared);

   ---------------------------------------------------------------------------
   -- Proof Verification
   ---------------------------------------------------------------------------

   function Verify_Obliteration_Proof
     (Proof : Obliteration_Proof) return Boolean
   with
     SPARK_Mode,
     Post => Verify_Obliteration_Proof'Result = Is_Valid_Obliteration (Proof);

   ---------------------------------------------------------------------------
   -- Query Functions
   ---------------------------------------------------------------------------

   function Get_Obliteration_Count return Obliteration_Count
   with
     SPARK_Mode,
     Global => (Input => Obliteration_Next),
     Post => Get_Obliteration_Count'Result = Obliteration_Next;

   function Is_Content_Obliterated (Hash : Content_Hash) return Boolean
   with
     SPARK_Mode,
     Global => (Input => (Obliterations, Obliteration_Next)),
     Post => Is_Content_Obliterated'Result = Was_Obliterated (Hash);

   ---------------------------------------------------------------------------
   -- GDPR Article 17 Compliance
   ---------------------------------------------------------------------------

   -- This type represents a formal erasure request
   type Erasure_Request is record
      Data_Subject_ID : Natural;       -- Anonymized identifier
      Content_Hash    : Content_Hash;  -- What to erase
      Legal_Basis     : Natural;       -- Reference to legal basis code
      Request_Time    : Natural;       -- Unix timestamp
   end record;

   procedure Process_Erasure_Request
     (Request : in     Erasure_Request;
      Proof   :    out Obliteration_Proof;
      Success :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => (Content_Store, Obliterations, Obliteration_Next)),
     Pre    => Is_Valid_Hash (Request.Content_Hash) and
               Content_Exists (Request.Content_Hash) and
               Obliteration_Next < Max_Obliterations,
     Post   => (if Success then
                  -- Content erased per GDPR Article 17
                  not Content_Exists (Request.Content_Hash) and
                  -- Proof of erasure generated
                  Has_Valid_Proof (Request.Content_Hash) and
                  -- Can demonstrate compliance
                  Is_Valid_Obliteration (Proof));

   ---------------------------------------------------------------------------
   -- Batch Obliteration
   ---------------------------------------------------------------------------

   type Hash_Array is array (Positive range <>) of Content_Hash;

   procedure Obliterate_Batch
     (Hashes       : in     Hash_Array;
      Reason       : in     String;
      Success_Count:    out Natural;
      All_Success  :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => (Content_Store, Obliterations, Obliteration_Next)),
     Pre    => Hashes'Length > 0 and
               Hashes'Length <= Max_Obliterations - Obliteration_Next and
               (for all I in Hashes'Range => Is_Valid_Hash (Hashes (I))),
     Post   => Success_Count <= Hashes'Length and
               (if All_Success then
                  Success_Count = Hashes'Length and
                  (for all I in Hashes'Range =>
                     Was_Obliterated (Hashes (I))));

end JanusKey_Obliteration;
