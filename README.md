# Rust Roguelike
This Rust Roguelike is a Roguelike written in Rust.


This is tested on Windows, and can be made to work on Linux. A simple
```bash
cargo run
```
should download all dependecies and get it running. If you are playing and not developing,
consider
```bash
cargo run --release
```
to get a smoother experience, but not that it will take some time the first time it is
run (several minutes).

## Key Map
  * 0,1,2,3,4,6,7,8,9: directional movement, or a selection in a menu
  * 5: pass turn
  * Up/Down/Left/Right Arrow: directional movement
  * A: interact with environment (arm/disarm traps)
  * Q: exit game
  * G: pickup (get) item under the player
  * D: drop item
  * I: open inventory menu
  * Y: yell
  * V: explore all cells
  * Esc: exit current action
  * Tab: swap primary weapons if holding two primary weapons
  * T: god mode
  * X: increase movement speed (sneak -> walk -> run)
  * Z: Decrease movement speed (run -> walk -> sneak)
  * O: holding space creates an overlay with additional information about the current situation
  * BackQuote: open the developer console (currently disabled)
  * U: use the current primary weapon

In addition, there is a pair of input systems called the Chord and the Cursor.

The chord system involves holding control, optionally alt (for the alternative action)
and selecting either one of ZXCVB (which coorespond to your primary and secondary
inventory positions, and then your skills) or moving. The alternate action for
items is to throw them (as opposed to using them), and for a skill, depends on the
particular skill.

If an item or skill is selected, either press Space to apply, press 5 (to apply, but also
to have the action apply to the tile occupied by the player), or press a direction. For
skills which require a direction, such as throwing, then a direction on the numpad
or number keys can be used.


The cursor system uses a cursor to select a tile. Space can be used to 'apply' the cursor
action, which by default is to move towards a tile. The chord system can be used to
add actions to the cursor, which will apply when 'Space' is pressed.
This can, for example, be used to throw an item on a particular tile instead of in a
direction.


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
messages in the messaging system that is used to execute a turn of the game.


Once the state system has created messages to modify the game, and optionally
changed its state, the resolution system starts.  This executes each message,
using an item, moving, using a skill, etc, which may in turn spawn more
messages, until there are no more messages to process.

Message processing may set the took\_turn flag for an entitity, and if the
player is marked as having taken a turn then the other entities in the game get
a chance to spawn messages. These messages are then themselves resolved until
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

