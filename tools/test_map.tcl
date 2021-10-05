source tools/commands.tcl

start_game -m empty

set_tile_walls 3 3 Empty ShortWall ShortWall
set_tile_walls 4 2 Wall Empty Empty

spawn Gol 4 3

