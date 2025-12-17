-- SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
-- SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
--
-- JanusKey SPARK Types: Formally verified type definitions
-- These types are designed to be provable by SPARK

pragma SPARK_Mode (On);

package JanusKey_Types is

   ---------------------------------------------------------------------------
   -- Hash Types (SHA256)
   ---------------------------------------------------------------------------

   subtype Hash_Index is Positive range 1 .. 64;
   subtype Hash_Char is Character range '0' .. 'f';
   type Content_Hash is array (Hash_Index) of Hash_Char;

   Null_Hash : constant Content_Hash := (others => '0');

   function Is_Valid_Hash (H : Content_Hash) return Boolean is
     (for all I in Hash_Index => H (I) in '0' .. '9' | 'a' .. 'f');

   ---------------------------------------------------------------------------
   -- Path Types
   ---------------------------------------------------------------------------

   Max_Path_Length : constant := 4096;
   subtype Path_Length is Natural range 0 .. Max_Path_Length;
   subtype Path_Index is Positive range 1 .. Max_Path_Length;

   type File_Path is record
      Data   : String (1 .. Max_Path_Length);
      Length : Path_Length;
   end record;

   function Is_Valid_Path (P : File_Path) return Boolean is
     (P.Length > 0 and P.Length <= Max_Path_Length);

   Null_Path : constant File_Path :=
     (Data => (others => ' '), Length => 0);

   ---------------------------------------------------------------------------
   -- Operation Types
   ---------------------------------------------------------------------------

   type Operation_Kind is
     (Op_Create,
      Op_Delete,
      Op_Modify,
      Op_Move,
      Op_Copy,
      Op_Chmod,
      Op_Mkdir,
      Op_Rmdir,
      Op_Symlink,
      Op_Append,
      Op_Truncate,
      Op_Touch);

   -- Operations that are always reversible (RMR)
   function Is_Reversible (Kind : Operation_Kind) return Boolean is
     (Kind in Op_Create | Op_Delete | Op_Modify | Op_Move |
              Op_Copy | Op_Chmod | Op_Mkdir | Op_Rmdir |
              Op_Symlink | Op_Append | Op_Truncate | Op_Touch);

   ---------------------------------------------------------------------------
   -- Operation Metadata (sufficient for reversal)
   ---------------------------------------------------------------------------

   type Operation_ID is new Positive;

   type Operation_Metadata is record
      ID              : Operation_ID;
      Kind            : Operation_Kind;
      Path            : File_Path;
      Secondary_Path  : File_Path;          -- For Move/Copy
      Content_Hash    : Content_Hash;       -- Original content hash
      New_Content_Hash: Content_Hash;       -- New content hash (Modify)
      Original_Size   : Natural;            -- For Append/Truncate
      Is_Undone       : Boolean;
   end record;

   function Has_Sufficient_Metadata (Op : Operation_Metadata) return Boolean is
     (case Op.Kind is
        when Op_Delete   => Is_Valid_Hash (Op.Content_Hash),
        when Op_Modify   => Is_Valid_Hash (Op.Content_Hash) and
                           Is_Valid_Hash (Op.New_Content_Hash),
        when Op_Move     => Is_Valid_Path (Op.Secondary_Path),
        when Op_Copy     => Is_Valid_Path (Op.Secondary_Path),
        when Op_Append   => Op.Original_Size > 0,
        when Op_Truncate => Is_Valid_Hash (Op.Content_Hash),
        when others      => True);

   ---------------------------------------------------------------------------
   -- File System State (Abstract Model)
   ---------------------------------------------------------------------------

   Max_Files : constant := 10_000;
   subtype File_Count is Natural range 0 .. Max_Files;
   subtype File_Index is Positive range 1 .. Max_Files;

   type File_Entry is record
      Path    : File_Path;
      Hash    : Content_Hash;
      Exists  : Boolean;
      Size    : Natural;
   end record;

   type File_System_State is array (File_Index) of File_Entry;

   ---------------------------------------------------------------------------
   -- Content Store State (Abstract Model)
   ---------------------------------------------------------------------------

   Max_Contents : constant := 100_000;
   subtype Content_Count is Natural range 0 .. Max_Contents;
   subtype Content_Index is Positive range 1 .. Max_Contents;

   type Content_Entry is record
      Hash   : Content_Hash;
      Stored : Boolean;
   end record;

   type Content_Store_State is array (Content_Index) of Content_Entry;

   function Content_Available
     (Store : Content_Store_State;
      Hash  : Content_Hash) return Boolean
   is (for some I in Content_Index =>
         Store (I).Hash = Hash and Store (I).Stored)
   with Ghost;

   ---------------------------------------------------------------------------
   -- Obliteration Proof (RMO)
   ---------------------------------------------------------------------------

   type Obliteration_Proof is record
      Content_Hash     : Content_Hash;
      Timestamp        : Natural;  -- Unix timestamp
      Nonce            : Content_Hash;  -- Random nonce
      Commitment       : Content_Hash;  -- H(hash || nonce || timestamp)
      Overwrite_Passes : Positive;
      Storage_Cleared  : Boolean;
   end record;

   function Is_Valid_Obliteration (Proof : Obliteration_Proof) return Boolean is
     (Proof.Storage_Cleared and
      Proof.Overwrite_Passes >= 3 and
      Is_Valid_Hash (Proof.Commitment));

end JanusKey_Types;
