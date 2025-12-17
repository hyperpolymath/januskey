-- SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
-- SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
--
-- JanusKey SPARK Operations: Implementation
--
-- NOTE: These implementations are stubs that model the behavior.
-- Actual file operations would call the Rust FFI layer.
-- SPARK verification focuses on the contract satisfaction.

pragma SPARK_Mode (On);

package body JanusKey_Operations is

   ---------------------------------------------------------------------------
   -- Helper: Find file index by path
   ---------------------------------------------------------------------------

   function Find_File (Path : File_Path) return File_Index
   with
     SPARK_Mode,
     Pre => File_Exists_At (Path),
     Post => Current_FS (Find_File'Result).Path = Path and
             Current_FS (Find_File'Result).Exists
   is
   begin
      for I in File_Index loop
         if Current_FS (I).Path = Path and Current_FS (I).Exists then
            return I;
         end if;
         pragma Loop_Invariant
           (for all J in File_Index'First .. I =>
              not (Current_FS (J).Path = Path and Current_FS (J).Exists) or J = I);
      end loop;
      -- Unreachable due to precondition
      return File_Index'First;
   end Find_File;

   ---------------------------------------------------------------------------
   -- Helper: Find free slot in file system
   ---------------------------------------------------------------------------

   function Find_Free_Slot return File_Index
   with
     SPARK_Mode,
     Post => not Current_FS (Find_Free_Slot'Result).Exists
   is
   begin
      for I in File_Index loop
         if not Current_FS (I).Exists then
            return I;
         end if;
      end loop;
      -- If no free slot, return first (will fail operation)
      return File_Index'First;
   end Find_Free_Slot;

   ---------------------------------------------------------------------------
   -- Helper: Store content in content store
   ---------------------------------------------------------------------------

   procedure Store_Content
     (Hash    : in     Content_Hash;
      Success :    out Boolean)
   with
     SPARK_Mode,
     Global => (In_Out => Content_Store),
     Post => (if Success then Content_Available (Content_Store, Hash))
   is
   begin
      for I in Content_Index loop
         if not Content_Store (I).Stored then
            Content_Store (I) := (Hash => Hash, Stored => True);
            Success := True;
            return;
         end if;
      end loop;
      Success := False;
   end Store_Content;

   ---------------------------------------------------------------------------
   -- Execute_Delete
   ---------------------------------------------------------------------------

   procedure Execute_Delete
     (Path     : in     File_Path;
      Metadata : in Out Operation_Metadata;
      Success  :    out Boolean)
   is
      Idx : File_Index;
      Store_OK : Boolean;
   begin
      Idx := Find_File (Path);

      -- Store original content hash
      Metadata.Content_Hash := Current_FS (Idx).Hash;
      Metadata.Kind := Op_Delete;
      Metadata.Path := Path;
      Metadata.Is_Undone := False;

      -- Store content for potential undo
      Store_Content (Current_FS (Idx).Hash, Store_OK);

      if Store_OK then
         -- Mark file as deleted
         Current_FS (Idx).Exists := False;
         Success := True;
      else
         Success := False;
      end if;
   end Execute_Delete;

   ---------------------------------------------------------------------------
   -- Execute_Create
   ---------------------------------------------------------------------------

   procedure Execute_Create
     (Path    : in     File_Path;
      Hash    : in     Content_Hash;
      Success :    out Boolean)
   is
      Idx : File_Index;
   begin
      Idx := Find_Free_Slot;

      if not Current_FS (Idx).Exists then
         Current_FS (Idx) := (Path   => Path,
                              Hash   => Hash,
                              Exists => True,
                              Size   => 0);
         Success := True;
      else
         Success := False;
      end if;
   end Execute_Create;

   ---------------------------------------------------------------------------
   -- Execute_Modify
   ---------------------------------------------------------------------------

   procedure Execute_Modify
     (Path         : in     File_Path;
      New_Hash     : in     Content_Hash;
      Metadata     : in Out Operation_Metadata;
      Success      :    out Boolean)
   is
      Idx : File_Index;
      Store_OK : Boolean;
   begin
      Idx := Find_File (Path);

      -- Record original state
      Metadata.Content_Hash := Current_FS (Idx).Hash;
      Metadata.New_Content_Hash := New_Hash;
      Metadata.Kind := Op_Modify;
      Metadata.Path := Path;
      Metadata.Is_Undone := False;

      -- Store original content for undo
      Store_Content (Current_FS (Idx).Hash, Store_OK);

      if Store_OK then
         -- Update file with new content
         Current_FS (Idx).Hash := New_Hash;
         Success := True;
      else
         Success := False;
      end if;
   end Execute_Modify;

   ---------------------------------------------------------------------------
   -- Execute_Move
   ---------------------------------------------------------------------------

   procedure Execute_Move
     (Source      : in     File_Path;
      Destination : in     File_Path;
      Metadata    : in Out Operation_Metadata;
      Success     :    out Boolean)
   is
      Src_Idx : File_Index;
      Dst_Idx : File_Index;
   begin
      Src_Idx := Find_File (Source);
      Dst_Idx := Find_Free_Slot;

      -- Record metadata for undo
      Metadata.Kind := Op_Move;
      Metadata.Path := Source;
      Metadata.Secondary_Path := Destination;
      Metadata.Is_Undone := False;

      if not Current_FS (Dst_Idx).Exists then
         -- Create file at destination with same content
         Current_FS (Dst_Idx) := (Path   => Destination,
                                  Hash   => Current_FS (Src_Idx).Hash,
                                  Exists => True,
                                  Size   => Current_FS (Src_Idx).Size);
         -- Remove source
         Current_FS (Src_Idx).Exists := False;
         Success := True;
      else
         Success := False;
      end if;
   end Execute_Move;

   ---------------------------------------------------------------------------
   -- Undo_Operation: THE CORE REVERSIBILITY IMPLEMENTATION
   ---------------------------------------------------------------------------

   procedure Undo_Operation
     (Metadata : in     Operation_Metadata;
      Success  :    out Boolean)
   is
   begin
      case Metadata.Kind is
         when Op_Delete =>
            -- Restore deleted file from content store
            Execute_Create (Metadata.Path, Metadata.Content_Hash, Success);

         when Op_Create =>
            -- Delete the created file
            declare
               Dummy_Meta : Operation_Metadata := Metadata;
            begin
               Execute_Delete (Metadata.Path, Dummy_Meta, Success);
            end;

         when Op_Modify =>
            -- Restore original content
            declare
               Idx : File_Index;
            begin
               if File_Exists_At (Metadata.Path) then
                  Idx := Find_File (Metadata.Path);
                  Current_FS (Idx).Hash := Metadata.Content_Hash;
                  Success := True;
               else
                  Success := False;
               end if;
            end;

         when Op_Move =>
            -- Move back: destination -> source
            declare
               Dst_Idx : File_Index;
               Src_Idx : File_Index;
            begin
               if File_Exists_At (Metadata.Secondary_Path) then
                  Dst_Idx := Find_File (Metadata.Secondary_Path);
                  Src_Idx := Find_Free_Slot;

                  if not Current_FS (Src_Idx).Exists then
                     -- Restore at original location
                     Current_FS (Src_Idx) :=
                       (Path   => Metadata.Path,
                        Hash   => Current_FS (Dst_Idx).Hash,
                        Exists => True,
                        Size   => Current_FS (Dst_Idx).Size);
                     -- Remove from destination
                     Current_FS (Dst_Idx).Exists := False;
                     Success := True;
                  else
                     Success := False;
                  end if;
               else
                  Success := False;
               end if;
            end;

         when Op_Copy =>
            -- Delete the copy
            declare
               Dummy_Meta : Operation_Metadata := Metadata;
            begin
               if File_Exists_At (Metadata.Secondary_Path) then
                  Dummy_Meta.Path := Metadata.Secondary_Path;
                  Execute_Delete (Metadata.Secondary_Path, Dummy_Meta, Success);
               else
                  Success := False;
               end if;
            end;

         when others =>
            -- Not yet implemented
            Success := False;
      end case;
   end Undo_Operation;

   ---------------------------------------------------------------------------
   -- Undo_Sequence: Undo operations in reverse order
   ---------------------------------------------------------------------------

   procedure Undo_Sequence
     (Ops     : in     Operation_Sequence;
      Success :    out Boolean)
   is
      Op_Success : Boolean;
   begin
      Success := True;

      -- Undo in reverse order
      for I in reverse Ops'Range loop
         Undo_Operation (Ops (I), Op_Success);
         if not Op_Success then
            Success := False;
            return;
         end if;
         pragma Loop_Invariant (Success);
      end loop;
   end Undo_Sequence;

end JanusKey_Operations;
