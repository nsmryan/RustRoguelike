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
(GameSettings), but does not modify the game's data (GameData). It can place
messages in the messaging system that is used to execute a turn of the game.


Once the state system has created messages to modify the game,
and optionally changed its state, the resolution
system starts.  This executes each message, using an item, moving, using a
skill, etc, which may in turn spawn more messages, until there are no more
messages to process. Then, the other entities in the game get a chance to spawn
messages, which are themselves resolved until no messages are left.
This is the only place where game actions occur- there are places that
generate maps or do other modifications, but each turn is entirely
a sequence of messages resolved in order.

