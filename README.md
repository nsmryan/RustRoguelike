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
  * Space: holding space creates an overlay with additional information about the current situation
  * BackQuote: open the developer console (currently disabled)
  * U: use the current primary weapon
