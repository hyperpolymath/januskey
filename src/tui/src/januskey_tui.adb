-- SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
-- SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
--
-- JanusKey TUI: Terminal User Interface for Reversible File Operations
-- Main entry point for the ncurses-based interface

with Ada.Text_IO;
with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Directories;
with Interfaces.C;
with Interfaces.C.Strings;

procedure JanusKey_TUI is

   package IO renames Ada.Text_IO;
   package CLI renames Ada.Command_Line;
   package C renames Interfaces.C;
   package CStr renames Interfaces.C.Strings;

   ---------------------------------------------------------------------------
   -- NCurses Bindings (thin bindings to libncurses)
   ---------------------------------------------------------------------------

   type Window_Ptr is new C.long;
   Null_Window : constant Window_Ptr := 0;

   -- NCurses constants
   KEY_UP    : constant C.int := 259;
   KEY_DOWN  : constant C.int := 258;
   KEY_LEFT  : constant C.int := 260;
   KEY_RIGHT : constant C.int := 261;
   KEY_ENTER : constant C.int := 10;
   KEY_ESC   : constant C.int := 27;
   KEY_TAB   : constant C.int := 9;
   KEY_F1    : constant C.int := 265;
   KEY_Q     : constant C.int := Character'Pos ('q');
   KEY_R     : constant C.int := Character'Pos ('r');
   KEY_O     : constant C.int := Character'Pos ('o');
   KEY_H     : constant C.int := Character'Pos ('h');

   -- Color pairs
   COLOR_HEADER    : constant C.int := 1;
   COLOR_SELECTED  : constant C.int := 2;
   COLOR_STATUS    : constant C.int := 3;
   COLOR_WARNING   : constant C.int := 4;
   COLOR_SUCCESS   : constant C.int := 5;
   COLOR_BORDER    : constant C.int := 6;

   -- NCurses functions
   function Initscr return Window_Ptr
     with Import, Convention => C, External_Name => "initscr";

   function Endwin return C.int
     with Import, Convention => C, External_Name => "endwin";

   function Cbreak return C.int
     with Import, Convention => C, External_Name => "cbreak";

   function Noecho return C.int
     with Import, Convention => C, External_Name => "noecho";

   function Keypad (Win : Window_Ptr; Bf : C.int) return C.int
     with Import, Convention => C, External_Name => "keypad";

   function Start_Color return C.int
     with Import, Convention => C, External_Name => "start_color";

   function Init_Pair (Pair : C.int; Fg : C.int; Bg : C.int) return C.int
     with Import, Convention => C, External_Name => "init_pair";

   function Color_Pair (N : C.int) return C.int
     with Import, Convention => C, External_Name => "COLOR_PAIR";

   function Curs_Set (Visibility : C.int) return C.int
     with Import, Convention => C, External_Name => "curs_set";

   function Getmaxy (Win : Window_Ptr) return C.int
     with Import, Convention => C, External_Name => "getmaxy";

   function Getmaxx (Win : Window_Ptr) return C.int
     with Import, Convention => C, External_Name => "getmaxx";

   function Wgetch (Win : Window_Ptr) return C.int
     with Import, Convention => C, External_Name => "wgetch";

   function Mvwaddstr (Win : Window_Ptr; Y, X : C.int;
                       Str : CStr.chars_ptr) return C.int
     with Import, Convention => C, External_Name => "mvwaddstr";

   function Wattron (Win : Window_Ptr; Attrs : C.int) return C.int
     with Import, Convention => C, External_Name => "wattron";

   function Wattroff (Win : Window_Ptr; Attrs : C.int) return C.int
     with Import, Convention => C, External_Name => "wattroff";

   function Wclear (Win : Window_Ptr) return C.int
     with Import, Convention => C, External_Name => "wclear";

   function Wrefresh (Win : Window_Ptr) return C.int
     with Import, Convention => C, External_Name => "wrefresh";

   function Newwin (Nlines, Ncols, Begin_Y, Begin_X : C.int) return Window_Ptr
     with Import, Convention => C, External_Name => "newwin";

   function Delwin (Win : Window_Ptr) return C.int
     with Import, Convention => C, External_Name => "delwin";

   function Box (Win : Window_Ptr; Verch, Horch : C.int) return C.int
     with Import, Convention => C, External_Name => "box";

   function Wbkgd (Win : Window_Ptr; Ch : C.int) return C.int
     with Import, Convention => C, External_Name => "wbkgd";

   function Mvwhline (Win : Window_Ptr; Y, X : C.int;
                      Ch : C.int; N : C.int) return C.int
     with Import, Convention => C, External_Name => "mvwhline";

   function Has_Colors return C.int
     with Import, Convention => C, External_Name => "has_colors";

   function Refresh return C.int
     with Import, Convention => C, External_Name => "refresh";

   -- Standard window
   Stdscr : Window_Ptr := Null_Window;
   pragma Import (C, Stdscr, "stdscr");

   -- Color constants
   COLOR_BLACK   : constant C.int := 0;
   COLOR_RED     : constant C.int := 1;
   COLOR_GREEN   : constant C.int := 2;
   COLOR_YELLOW  : constant C.int := 3;
   COLOR_BLUE    : constant C.int := 4;
   COLOR_MAGENTA : constant C.int := 5;
   COLOR_CYAN    : constant C.int := 6;
   COLOR_WHITE   : constant C.int := 7;

   -- Attribute constants
   A_BOLD    : constant C.int := 2097152;  -- 1 << 21
   A_REVERSE : constant C.int := 262144;   -- 1 << 18

   ---------------------------------------------------------------------------
   -- TUI State Types
   ---------------------------------------------------------------------------

   type Panel_Type is (Panel_Operations, Panel_History, Panel_Files);

   type Menu_Item is record
      Label       : Unbounded_String;
      Description : Unbounded_String;
      Key         : Character;
   end record;

   type Operation_Record is record
      Timestamp   : Unbounded_String;
      Op_Type     : Unbounded_String;
      Path        : Unbounded_String;
      Reversible  : Boolean;
   end record;

   -- Menu items for operations panel
   Operations_Menu : constant array (1 .. 8) of Menu_Item := (
      (To_Unbounded_String ("  [R] Reverse Operation"),
       To_Unbounded_String ("Undo last reversible operation"), 'R'),
      (To_Unbounded_String ("  [O] Obliterate"),
       To_Unbounded_String ("Permanently delete (GDPR Art. 17)"), 'O'),
      (To_Unbounded_String ("  [H] History"),
       To_Unbounded_String ("View operation history"), 'H'),
      (To_Unbounded_String ("  [S] Status"),
       To_Unbounded_String ("Show JanusKey status"), 'S'),
      (To_Unbounded_String ("  [I] Init"),
       To_Unbounded_String ("Initialize JanusKey in directory"), 'I'),
      (To_Unbounded_String ("  [V] Verify"),
       To_Unbounded_String ("Verify content integrity"), 'V'),
      (To_Unbounded_String ("  [C] Config"),
       To_Unbounded_String ("Edit configuration"), 'C'),
      (To_Unbounded_String ("  [Q] Quit"),
       To_Unbounded_String ("Exit JanusKey TUI"), 'Q')
   );

   -- Sample history (in real impl, loaded from .januskey/metadata.json)
   History : constant array (1 .. 5) of Operation_Record := (
      (To_Unbounded_String ("2025-01-15 10:30:22"),
       To_Unbounded_String ("RMR:CREATE"),
       To_Unbounded_String ("./src/main.rs"),
       True),
      (To_Unbounded_String ("2025-01-15 10:28:15"),
       To_Unbounded_String ("RMR:MODIFY"),
       To_Unbounded_String ("./Cargo.toml"),
       True),
      (To_Unbounded_String ("2025-01-15 10:25:00"),
       To_Unbounded_String ("RMO:WIPE"),
       To_Unbounded_String ("./secrets.txt"),
       False),
      (To_Unbounded_String ("2025-01-15 10:20:33"),
       To_Unbounded_String ("RMR:DELETE"),
       To_Unbounded_String ("./old_config.json"),
       True),
      (To_Unbounded_String ("2025-01-15 10:15:00"),
       To_Unbounded_String ("RMR:RENAME"),
       To_Unbounded_String ("./data.bak -> ./data.json"),
       True)
   );

   ---------------------------------------------------------------------------
   -- TUI State Variables
   ---------------------------------------------------------------------------

   Main_Win     : Window_Ptr := Null_Window;
   Ops_Win      : Window_Ptr := Null_Window;
   History_Win  : Window_Ptr := Null_Window;
   Status_Win   : Window_Ptr := Null_Window;

   Max_Y, Max_X : C.int := 0;
   Active_Panel : Panel_Type := Panel_Operations;
   Selected_Idx : Natural := 1;
   Running      : Boolean := True;

   Working_Dir  : Unbounded_String := To_Unbounded_String (".");

   ---------------------------------------------------------------------------
   -- Helper Procedures
   ---------------------------------------------------------------------------

   procedure Put_String (Win : Window_Ptr; Y, X : C.int; S : String) is
      C_Str : CStr.chars_ptr := CStr.New_String (S);
      Dummy : C.int;
   begin
      Dummy := Mvwaddstr (Win, Y, X, C_Str);
      CStr.Free (C_Str);
   end Put_String;

   procedure Draw_Header is
      Title : constant String := " JanusKey - Reversible File Operations ";
      Dummy : C.int;
   begin
      Dummy := Wattron (Main_Win, Color_Pair (COLOR_HEADER) + A_BOLD);
      Put_String (Main_Win, 0, (Max_X - C.int (Title'Length)) / 2, Title);
      Dummy := Wattroff (Main_Win, Color_Pair (COLOR_HEADER) + A_BOLD);

      -- Subtitle
      Put_String (Main_Win, 1, 2,
                  "Dir: " & To_String (Working_Dir) &
                  " | Tab: Switch Panel | F1: Help | Q: Quit");
   end Draw_Header;

   procedure Draw_Operations_Panel is
      Panel_Y      : constant C.int := 3;
      Panel_X      : constant C.int := 1;
      Panel_Width  : constant C.int := Max_X / 2 - 2;
      Panel_Height : constant C.int := Max_Y - 6;
      Dummy        : C.int;
   begin
      -- Create or recreate window
      if Ops_Win /= Null_Window then
         Dummy := Delwin (Ops_Win);
      end if;
      Ops_Win := Newwin (Panel_Height, Panel_Width, Panel_Y, Panel_X);

      -- Draw border
      if Active_Panel = Panel_Operations then
         Dummy := Wattron (Ops_Win, Color_Pair (COLOR_SELECTED));
      else
         Dummy := Wattron (Ops_Win, Color_Pair (COLOR_BORDER));
      end if;
      Dummy := Box (Ops_Win, 0, 0);
      Dummy := Wattroff (Ops_Win, Color_Pair (COLOR_SELECTED));
      Dummy := Wattroff (Ops_Win, Color_Pair (COLOR_BORDER));

      -- Panel title
      Put_String (Ops_Win, 0, 2, " Operations ");

      -- Menu items
      for I in Operations_Menu'Range loop
         if Active_Panel = Panel_Operations and then I = Selected_Idx then
            Dummy := Wattron (Ops_Win, A_REVERSE);
         end if;

         Put_String (Ops_Win, C.int (I) + 1, 1,
                     To_String (Operations_Menu (I).Label));

         if Active_Panel = Panel_Operations and then I = Selected_Idx then
            Dummy := Wattroff (Ops_Win, A_REVERSE);
            -- Show description for selected item
            Put_String (Ops_Win, Panel_Height - 2, 2,
                        To_String (Operations_Menu (I).Description));
         end if;
      end loop;

      Dummy := Wrefresh (Ops_Win);
   end Draw_Operations_Panel;

   procedure Draw_History_Panel is
      Panel_Y      : constant C.int := 3;
      Panel_X      : constant C.int := Max_X / 2;
      Panel_Width  : constant C.int := Max_X / 2 - 1;
      Panel_Height : constant C.int := Max_Y - 6;
      Dummy        : C.int;
      Line         : C.int := 2;
   begin
      -- Create or recreate window
      if History_Win /= Null_Window then
         Dummy := Delwin (History_Win);
      end if;
      History_Win := Newwin (Panel_Height, Panel_Width, Panel_Y, Panel_X);

      -- Draw border
      if Active_Panel = Panel_History then
         Dummy := Wattron (History_Win, Color_Pair (COLOR_SELECTED));
      else
         Dummy := Wattron (History_Win, Color_Pair (COLOR_BORDER));
      end if;
      Dummy := Box (History_Win, 0, 0);
      Dummy := Wattroff (History_Win, Color_Pair (COLOR_SELECTED));
      Dummy := Wattroff (History_Win, Color_Pair (COLOR_BORDER));

      -- Panel title
      Put_String (History_Win, 0, 2, " Recent Operations ");

      -- Column headers
      Dummy := Wattron (History_Win, A_BOLD);
      Put_String (History_Win, 1, 2, "Time       Type        Path");
      Dummy := Wattroff (History_Win, A_BOLD);

      -- History items
      for I in History'Range loop
         declare
            Rec : Operation_Record renames History (I);
            Time_Str : constant String :=
              To_String (Rec.Timestamp) (12 .. 19);  -- Just time portion
         begin
            -- Color based on reversibility
            if Rec.Reversible then
               Dummy := Wattron (History_Win, Color_Pair (COLOR_SUCCESS));
            else
               Dummy := Wattron (History_Win, Color_Pair (COLOR_WARNING));
            end if;

            Put_String (History_Win, Line, 2,
                        Time_Str & "  " &
                        To_String (Rec.Op_Type) (1 .. 10));

            -- Truncate path if needed
            declare
               Path : constant String := To_String (Rec.Path);
               Max_Path_Len : constant Natural :=
                 Natural (Panel_Width) - 26;
               Display_Path : constant String :=
                 (if Path'Length > Max_Path_Len
                  then "..." & Path (Path'Last - Max_Path_Len + 4 .. Path'Last)
                  else Path);
            begin
               Put_String (History_Win, Line, 24, Display_Path);
            end;

            Dummy := Wattroff (History_Win, Color_Pair (COLOR_SUCCESS));
            Dummy := Wattroff (History_Win, Color_Pair (COLOR_WARNING));

            Line := Line + 1;
         end;
      end loop;

      -- Legend
      Put_String (History_Win, Panel_Height - 2, 2,
                  "Green=Reversible  Yellow=Obliterated");

      Dummy := Wrefresh (History_Win);
   end Draw_History_Panel;

   procedure Draw_Status_Bar is
      Status_Y : constant C.int := Max_Y - 2;
      Dummy    : C.int;
   begin
      -- Create or recreate window
      if Status_Win /= Null_Window then
         Dummy := Delwin (Status_Win);
      end if;
      Status_Win := Newwin (2, Max_X, Status_Y, 0);

      Dummy := Wbkgd (Status_Win, Color_Pair (COLOR_STATUS));

      -- Status line
      Put_String (Status_Win, 0, 2,
                  "JanusKey v1.0.0 | RMR: 42 ops | RMO: 3 wipes | " &
                  "Storage: 15.2 MB");
      Put_String (Status_Win, 1, 2,
                  "Press F1 for help | " &
                  "github.com/hyperpolymath/januskey");

      Dummy := Wrefresh (Status_Win);
   end Draw_Status_Bar;

   procedure Draw_All is
      Dummy : C.int;
   begin
      Dummy := Wclear (Main_Win);

      -- Update dimensions
      Max_Y := Getmaxy (Main_Win);
      Max_X := Getmaxx (Main_Win);

      Draw_Header;
      Draw_Operations_Panel;
      Draw_History_Panel;
      Draw_Status_Bar;

      Dummy := Wrefresh (Main_Win);
   end Draw_All;

   procedure Handle_Key (Key : C.int) is
   begin
      case Key is
         when KEY_TAB =>
            -- Switch panels
            case Active_Panel is
               when Panel_Operations =>
                  Active_Panel := Panel_History;
               when Panel_History =>
                  Active_Panel := Panel_Operations;
               when Panel_Files =>
                  Active_Panel := Panel_Operations;
            end case;
            Selected_Idx := 1;

         when KEY_UP =>
            if Selected_Idx > 1 then
               Selected_Idx := Selected_Idx - 1;
            end if;

         when KEY_DOWN =>
            if Active_Panel = Panel_Operations then
               if Selected_Idx < Operations_Menu'Last then
                  Selected_Idx := Selected_Idx + 1;
               end if;
            elsif Active_Panel = Panel_History then
               if Selected_Idx < History'Last then
                  Selected_Idx := Selected_Idx + 1;
               end if;
            end if;

         when KEY_Q | KEY_ESC =>
            Running := False;

         when KEY_R =>
            -- Would invoke: jk reverse
            null;

         when KEY_O =>
            -- Would invoke: jk obliterate
            null;

         when KEY_H =>
            -- Would invoke: jk history
            null;

         when KEY_ENTER =>
            -- Execute selected operation
            if Active_Panel = Panel_Operations then
               case Selected_Idx is
                  when 8 =>  -- Quit
                     Running := False;
                  when others =>
                     null;  -- Would invoke corresponding jk command
               end case;
            end if;

         when others =>
            null;
      end case;
   end Handle_Key;

   procedure Show_Help is
      Help_Win : Window_Ptr;
      Dummy    : C.int;
      Key      : C.int;
      Help_H   : constant C.int := 16;
      Help_W   : constant C.int := 60;
   begin
      Help_Win := Newwin (Help_H, Help_W,
                          (Max_Y - Help_H) / 2,
                          (Max_X - Help_W) / 2);

      Dummy := Wbkgd (Help_Win, Color_Pair (COLOR_HEADER));
      Dummy := Box (Help_Win, 0, 0);

      Put_String (Help_Win, 0, 2, " JanusKey Help ");
      Put_String (Help_Win, 2, 2, "Navigation:");
      Put_String (Help_Win, 3, 4, "Tab       - Switch between panels");
      Put_String (Help_Win, 4, 4, "Up/Down   - Navigate menu items");
      Put_String (Help_Win, 5, 4, "Enter     - Execute selected operation");
      Put_String (Help_Win, 7, 2, "Operations:");
      Put_String (Help_Win, 8, 4, "R - Reverse last operation (RMR)");
      Put_String (Help_Win, 9, 4, "O - Obliterate file (RMO/GDPR)");
      Put_String (Help_Win, 10, 4, "H - View full history");
      Put_String (Help_Win, 11, 4, "S - Show status");
      Put_String (Help_Win, 13, 2, "Press any key to close...");

      Dummy := Wrefresh (Help_Win);
      Key := Wgetch (Help_Win);
      Dummy := Delwin (Help_Win);
   end Show_Help;

   ---------------------------------------------------------------------------
   -- Main
   ---------------------------------------------------------------------------

   Dummy : C.int;
   Key   : C.int;

begin
   -- Parse command line for working directory
   if CLI.Argument_Count > 0 then
      Working_Dir := To_Unbounded_String (CLI.Argument (1));
   else
      Working_Dir := To_Unbounded_String (Ada.Directories.Current_Directory);
   end if;

   -- Initialize ncurses
   Main_Win := Initscr;
   if Main_Win = Null_Window then
      IO.Put_Line ("Error: Failed to initialize ncurses");
      return;
   end if;

   Dummy := Cbreak;
   Dummy := Noecho;
   Dummy := Keypad (Main_Win, 1);
   Dummy := Curs_Set (0);  -- Hide cursor

   -- Initialize colors if available
   if Has_Colors /= 0 then
      Dummy := Start_Color;
      Dummy := Init_Pair (COLOR_HEADER, COLOR_WHITE, COLOR_BLUE);
      Dummy := Init_Pair (COLOR_SELECTED, COLOR_CYAN, COLOR_BLACK);
      Dummy := Init_Pair (COLOR_STATUS, COLOR_BLACK, COLOR_WHITE);
      Dummy := Init_Pair (COLOR_WARNING, COLOR_YELLOW, COLOR_BLACK);
      Dummy := Init_Pair (COLOR_SUCCESS, COLOR_GREEN, COLOR_BLACK);
      Dummy := Init_Pair (COLOR_BORDER, COLOR_WHITE, COLOR_BLACK);
   end if;

   -- Get initial dimensions
   Max_Y := Getmaxy (Main_Win);
   Max_X := Getmaxx (Main_Win);

   -- Main loop
   while Running loop
      Draw_All;
      Key := Wgetch (Main_Win);

      if Key = KEY_F1 then
         Show_Help;
      else
         Handle_Key (Key);
      end if;
   end loop;

   -- Cleanup
   if Ops_Win /= Null_Window then
      Dummy := Delwin (Ops_Win);
   end if;
   if History_Win /= Null_Window then
      Dummy := Delwin (History_Win);
   end if;
   if Status_Win /= Null_Window then
      Dummy := Delwin (Status_Win);
   end if;

   Dummy := Endwin;

   IO.Put_Line ("JanusKey TUI terminated.");

end JanusKey_TUI;
