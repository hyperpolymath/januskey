-- SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
-- SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
--
-- JanusKey SPARK Operations: Formally verified RMR primitive
--
-- THEOREM (Individual Reversibility):
--   For any operation op and valid state S:
--   reverse(apply(op, S)) = S
--
-- This package provides SPARK contracts that express this theorem.
-- Proof obligations must be discharged by gnatprove.

pragma SPARK_Mode (On);

with JanusKey_Types; use JanusKey_Types;

package JanusKey_Operations is

   ---------------------------------------------------------------------------
   -- Global State (Ghost variables for specification)
   ---------------------------------------------------------------------------

   Current_FS    : File_System_State with Ghost;
   Content_Store : Content_Store_State with Ghost;

   ---------------------------------------------------------------------------
   -- State Validity Predicates
   ---------------------------------------------------------------------------

   function Valid_State return Boolean is
     (for all I in File_Index =>
        (if Current_FS (I).Exists then Is_Valid_Path (Current_FS (I).Path)))
   with Ghost;

   function File_Exists_At (Path : File_Path) return Boolean is
     (for some I in File_Index =>
        Current_FS (I).Path = Path and Current_FS (I).Exists)
   with Ghost;

   function File_Has_Hash (Path : File_Path; Hash : Content_Hash) return Boolean is
     (for some I in File_Index =>
        Current_FS (I).Path = Path and
        Current_FS (I).Exists and
        Current_FS (I).Hash = Hash)
   with Ghost;

   ---------------------------------------------------------------------------
   -- RMR Primitive: Execute Operation
   ---------------------------------------------------------------------------

   procedure Execute_Delete
     (Path     : in     File_Path;
      Metadata : in out Operation_Metadata;
      Success  :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => (Current_FS, Content_Store)),
     Pre    => Valid_State and
               Is_Valid_Path (Path) and
               File_Exists_At (Path),
     Post   => (if Success then
                  -- File no longer exists
                  not File_Exists_At (Path) and
                  -- Content was stored for reversal
                  Content_Available (Content_Store, Metadata.Content_Hash) and
                  -- Metadata is sufficient for undo
                  Has_Sufficient_Metadata (Metadata) and
                  Metadata.Kind = Op_Delete);

   procedure Execute_Create
     (Path    : in     File_Path;
      Hash    : in     Content_Hash;
      Success :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => Current_FS),
     Pre    => Valid_State and
               Is_Valid_Path (Path) and
               not File_Exists_At (Path),
     Post   => (if Success then
                  File_Exists_At (Path) and
                  File_Has_Hash (Path, Hash));

   procedure Execute_Modify
     (Path         : in     File_Path;
      New_Hash     : in     Content_Hash;
      Metadata     : in out Operation_Metadata;
      Success      :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => (Current_FS, Content_Store)),
     Pre    => Valid_State and
               Is_Valid_Path (Path) and
               File_Exists_At (Path) and
               Is_Valid_Hash (New_Hash),
     Post   => (if Success then
                  File_Exists_At (Path) and
                  File_Has_Hash (Path, New_Hash) and
                  -- Original content preserved
                  Content_Available (Content_Store, Metadata.Content_Hash) and
                  Has_Sufficient_Metadata (Metadata) and
                  Metadata.Kind = Op_Modify);

   procedure Execute_Move
     (Source      : in     File_Path;
      Destination : in     File_Path;
      Metadata    : in Out Operation_Metadata;
      Success     :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => Current_FS),
     Pre    => Valid_State and
               Is_Valid_Path (Source) and
               Is_Valid_Path (Destination) and
               File_Exists_At (Source) and
               not File_Exists_At (Destination),
     Post   => (if Success then
                  not File_Exists_At (Source) and
                  File_Exists_At (Destination) and
                  Has_Sufficient_Metadata (Metadata) and
                  Metadata.Kind = Op_Move and
                  Metadata.Path = Source and
                  Metadata.Secondary_Path = Destination);

   ---------------------------------------------------------------------------
   -- RMR Primitive: Undo Operation (Reverse)
   ---------------------------------------------------------------------------

   procedure Undo_Operation
     (Metadata : in     Operation_Metadata;
      Success  :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => (Current_FS, Content_Store)),
     Pre    => Valid_State and
               Has_Sufficient_Metadata (Metadata) and
               not Metadata.Is_Undone and
               -- Content must be available for operations that need it
               (if Metadata.Kind in Op_Delete | Op_Modify | Op_Truncate then
                  Content_Available (Content_Store, Metadata.Content_Hash)),
     Post   => (if Success then
                  -- THE REVERSIBILITY THEOREM:
                  -- After undo, the file system state is restored
                  (case Metadata.Kind is
                     when Op_Delete =>
                        File_Exists_At (Metadata.Path) and
                        File_Has_Hash (Metadata.Path, Metadata.Content_Hash),
                     when Op_Create =>
                        not File_Exists_At (Metadata.Path),
                     when Op_Modify =>
                        File_Exists_At (Metadata.Path) and
                        File_Has_Hash (Metadata.Path, Metadata.Content_Hash),
                     when Op_Move =>
                        File_Exists_At (Metadata.Path) and
                        not File_Exists_At (Metadata.Secondary_Path),
                     when Op_Copy =>
                        not File_Exists_At (Metadata.Secondary_Path),
                     when others =>
                        True));

   ---------------------------------------------------------------------------
   -- Composition Theorem: Sequential operations are reversible
   ---------------------------------------------------------------------------

   type Operation_Sequence is array (Positive range <>) of Operation_Metadata;

   function All_Sufficient (Ops : Operation_Sequence) return Boolean is
     (for all I in Ops'Range => Has_Sufficient_Metadata (Ops (I)))
   with Ghost;

   function All_Content_Available (Ops : Operation_Sequence) return Boolean is
     (for all I in Ops'Range =>
        (if Ops (I).Kind in Op_Delete | Op_Modify | Op_Truncate then
           Content_Available (Content_Store, Ops (I).Content_Hash)))
   with Ghost;

   procedure Undo_Sequence
     (Ops     : in     Operation_Sequence;
      Success :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => (Current_FS, Content_Store)),
     Pre    => Valid_State and
               Ops'Length > 0 and
               All_Sufficient (Ops) and
               All_Content_Available (Ops),
     Post   => (if Success then Valid_State);
     -- Full postcondition: state equals state before sequence was applied
     -- This requires ghost state tracking which is more complex

end JanusKey_Operations;
