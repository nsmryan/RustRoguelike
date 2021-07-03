# Rust Roguelike

This Rust Roguelike is a Roguelike written in Rust. It is a Roguelike with a focus
on movement, use of space, stealth, and some resource management. 


Some unusual aspects that make this game interesting:

    * Intertile walls- walls that are between two tiles instead of within a tile
    * A varied movement system which allows slow sneaking movement, normal walking, and running,
    each with different levels of visibility, levels of noise, movement distance, and capabilities (such as
    jumping over short walls while running).
    * A stealth system based on noise level and visiblilty, with various factors effecting LoS.
    * Unusual classes like 'clockwork', 'monolith', 'grass', or 'heirophant' with their own unique skills.
    * A control scheme designed around simplicity and immediacy, while also providing a high level of
      information to the player to allow careful consideration of their moves.


## Building

This is tested on Windows and Linux. On Linux (and on a Mac, although
this is untested), make sure SDL2 is installed. On Windows the repository
contains the necessary SDL2 files.

Then just run:
```bash
cargo run
```
which will download all dependecies and get it running. If you are playing and not developing,
consider
```bash
cargo run --release
```
to get a smoother experience, but note that it will take some time the first time it is
run (several minutes).

## Gameplay

### Key Map

There are two gameplay modes: cursor mode and direct mode.


In direct mode, movement keys move the player and items can be used.

Cursor mode is entered using the 'space' key. In cursor mode the movement keys
move a cursor around the map. This cursor can be used to inspect tiles, use skills,
and throw items.

#### Movement

Move with the number keys. The arrow keys also work, but do not allow diagonal movement.

The number '5' key passes your turn.

#### Items

Items are mapped to 'z' (first item), 'x' (second item), and 'c' (third item).

Items can be used in multiple ways. Holding the item's key and pressing a direction
will use the item in that direction, such as to swing a hammer towards a wall or
golem.

Items can also be thrown by pressing their key while in cursor mode. This throws
them towards the cursor's location.


Items can be picked up with the 'g' key (to 'get' the item).

Items can be dropped by throwing them on the same tile that the player is on,
or using them in the '5' direction (the 'pass turn' key).


#### Skills

Skills are mapped to 'a' (first skill), 's' (second skill), and 'd' (third skill).
Skills can be used by pressing their key in cursor mode.

#### Menus

There are several menus that help you understand the game or select options.

The 'h' key opens the class menu, allowing you to select a class.

The 'j' key opens the skill menu, listing your current skills. 

The 'i' key opens the inventory menu, listing your current items. 

The 'esc' key can be used to exit a menu.


#### Other

Holding alt and pressing a directional key will 'interact' with the tile in that direction,
such as to disarm or arm a trap.

The 'o' key shows an information overlay while it is held. This shows golem Fov, attack positions,
and other information.

The 'y' will cause your character to yell, making noise.

The 'q' key will prompt to exit the game, and pressing 'q' again will exit.

The 't' key is a debugging key which makes you invincible and shows you the map. It
can be pressed again to hide the map.

The 'p' key is a debugging key which regenerates a new level.


### Sound

The game has a sound system in which different actions make different amounts of
sound. The golems may hear a sound and investigate its source, possibly causing them
to discover the player. Actions like running or yelling make a lot of sound, while
sneaking and using most skills make little to no sound.

Some skills effect the amount of sound movement takes, and the surface of a tile
can dampen sounds (grass), or make them louder (rubble).

### Traps

The game contains traps of various types. Walking on a trap triggers it if the 
trap is armed. Traps can be disarmed by interacting with them, and then armed
again by interacting again.

A disarmed trap can be picked up, allowing the player to carry traps around and
place and rearm them.

### Triggers

In addition to traps, there are stationary triggers which cannot be disarmed or
picked up.


## Architecture

The overall architecture of the game is something like this: inputs are
received from the system using SDL2, and translated into an internal InputEvent
structure (keypresses, mouse movement, etc).

This input event is translated into an action (InputAction) that describes what
the input does in the game's current state (without changing its state).
This translation is internal to the game, and uses the game's state.

This input action is provided to the step\_game function of the cleverly named
Game structure.  

The step\_game function dispatches through the game's current state- playing,
in a menu, etc. This can change the state of the game, and any settings
(GameSettings), but does not modify the game's data (GameData). It can only place
messages in the message queue that is used to execute a turn of the game.


Once the state system has created its messages, and optionally
changed its state, the resolution system starts.  This executes each message- 
using an item, moving, using a skill, etc, which may in turn spawn more
messages- until there are no more messages to process.

Message processing may set the took\_turn flag for an entitity, and if the
player is marked as having taken a turn then the other entities in the game get
a chance to spawn messages. These messages are themselves resolved until
no messages are left.

This is the only place where game actions occur- there are places that generate
maps or do other modifications, but each turn is entirely handled as a sequence
of messages resolved in order.


### Types

The main structures are: 

    * Game: this structure holds the GameData, as well as settings, configuration, logging, random
    number generation- generally data needed to run the game but not necessarily within the game world.
        * GameData: this structure is simply an Entities and a Map. It is the core structure of the game,
            holding the entire game world and all its entities. This structure provides some functions such
            as FOV which take into account both the map and the entities, where the lower level functions
            can't use both types of information.
            * Entities: this structure contains all components, and a vector of ids which identify
            each entity. The id can be used to index a component to get that entities data if it exists.
            * Map: this is a grid of Tile structures, with information on blocking movement, sight, the
            types of surfaces, etc. This structure has many functions for FOV, pathing, floodfill, and others.
        * Config: the game's static configuration, read from config.yaml.
        * MsgLog: the message log is used to both print a console log for the user (classic Roguelike style),
        as well as to drive the game's logic. The messages are processed by the game engine to change game
        state, as well as provided to the display system to change the display state.
    * InputAction: the input actions are all actions that the player can perform, including navigating menus
    or quitting the game.
    * Msg: the messages put in the message log. This contains all changes that can occur to the game state, 
      including movements, attacks, triggering traps, using skills, and starting and ending a turn.
    * Input: the state of the input system. This type contains information on modifier keys and which other
      keys are held, and uses this information to map key presses into inputs to the game.
    * Display: the display state contains SDL2 types, loaded textures, sprite and animation information, as well
      as screen layout. It is used in render the game to the screen.


There are a number of ancillary structures such as Vaults for parts of maps, GameSettings for mutable data like
the current turn or whether the overlay is on, ProcCmd for controlling level generation, and others.

### Design

There are some design considerations that have a sigificant effect on the game's code. One is that no
part of the codebase keeps links to other parts- it is all a sinlge big data structure (Game), which is
taken apart and passed to functions to mutate it and have side effects. This keeps changes to state
in constrained locations, which helps with finding the location and sequence of state changes.


In addition, many components do not make changes themselves, but emit structures that are then processed.
This allows things like recording actions, and testing components. A good example is the input system
processing inputs to game actions, and then to messages.


The game is also split into layered crates within a workspace, such that the roguelike\_core knows
about the core types, but not how the game is displayed or how maps are generated (for example),
while roguelike\_engine knows about actions, procedural generation, and handling inputs, but not
displaying or the main loop.


Finally roguelike\_main is the main loop, as well as the SDL2 display system. This is split into
display data with the animations, textures, screen layout, etc, and then a rendering function
which does all the drawing to the screen using the display data and game's state (Game).


This split allows roguelike\_lib to compile a binary that is separate from SDL2 and can be integrated
into other systems.


### Interesting Internal Features

There are a number of interesting features that are not necessarily visible when playing the game.


#### Winding and Rewinding Time

The game has a simple "undo/redo" system, implemented by copying the game state each time an action
is taken. This is not part of the gameplay, but rather a debugging tool.


These states are kept in a stack, and can be popped off with the '[' key. In addition to the
game's state, the action that caused a transition between game states is stored, so that when
the ']' key is pressed that action is replayed, allowing both forward and backwards movement.


One interesting nuance here is that you can rewind the game, take a different action, and then
replay your old actions on top of the new game. This allows debugging certain situations where
one path causes a problem and another does not (like using an item before making a move).


#### Command Line Interpreter and rl_engine

The game has a simple command line interface defined in commands.rs. When compiling the 'engine' version
of the game (the one used when running within Unity), this is the only interface, while in the SDL2
version of the game this interface is available in addition to the game GUI.


This interface exposes simple commands to list ids, query the map, change entity states, add entities,
etc. This can be driven by other programs such as TCL or Unity. The commands print out results in a
simple text format that must be parsed by the calling program. All interaction takes place using
stdin and stdout.


The rl\_engine version of the game runs exactly as the SDL2 version internally- the logic is
driven by the same input events. The difference is only in the source of those events, and the
lack of a UI when running the engine.


#### Performance Monitoring

The game generates performance logs as game.log. These contain some basic 'spans' such as the time
taken for logic, display, waiting for a new frame to start, etc. Additional timers can be added
to get more detail.


A pyimgui tool called analyzer.py can visualize these traces and plot them for analysis.

#### Map Density Heatmap

The game generates a file called map_emptiness_distribution.txt which contains distribution of
how densly populated the map is. This can be turned in to a heatmap with map_distribution.tcl.
This information is used for guiding procgen to make sure the maps are not to spare or
too dense.

#### Wave Function Collapse (WFC)

The game uses the WFC algorithm internally for map generation. The resources directory contains
some wfc_seed_*.png files. These images contain pixels used as input to the algorithm.

#### Symmetric Shadowcasting

The Line of Sight (LoS) algorithm used in this game uses the symmetric shadowcasting algoritm.
This is a very nice algorithm for LoS on a grid, and was adapted from a Python version
and turned into a separate Rust crate for use in this game.

#### Vaults

The game makes use of 'vault' files found in resources/vaults. These files contains small maps
in a text format with interesting formations of entities and tiles. These are stamped into the
map during procgen to create structured areas within the otherwise randomly generated maps.


The format of the vaults requires twice the number of character as the number of tiles in the
resulting map. This allows even tiles to indicate the contents of a tile, and odd tiles the intertile
walls.

