# Untitled Bandit

Untitled Bandit is a game made in pure rust primarily using [bandit][bandit]
and [crossterm][crossterm] for the convenient terminal handling.

## Gameplay 

The game is a grid based and turn based roguelike, with all enemies on screen acting only after the player takes their turn. A turn consititutes
a single movement (or null movement, also known as waiting a turn) of the player.

During the game, the player will move between rooms via doors, denoted with a '/' character. Many unexplored rooms will 
contain enemies (each represented with a different character of the latin alphabet), which must all be defeated before
exit is permitted. The attack patterns of specific enemies is up to the player to discover.

Occasionally, the player may encounter a door that is locked, denoted with a '╬' character. These can only be unlocked
using a key of the corresponding colour, which will be found elsewhere on the floor. To unlock a door, simply try to move
into it while holding a key of the correct colour. This will remove the key used, as well as the door. Rooms after a
locked door will contain a more powerful enemy, and an exit to the next floor. This exit is only accessible once the
enemy in the room is defeated.

Going to the next floor will place the player back at position (0, 0) and generate an entirely new map to explore.

### Combat

The player and all enemies have a health pool, which is reduced by incoming attacks. Once the health of an enemy reaches
0, it is defeated and removed from the screen. If the player's health reaches 0, the game stops, and will be reset to 
floor 0 on the next attempt.

Attacking is performed by attempting to move into an enemy. This will instead deal 1 damage to the target, reducing its
health by 1.

Enemies will try to do the same to the player, often moving towards them if the player is too far away to directly attack.
An enemy may attack on its next turn if it is highlighted red, and not all enemies have the same attack pattern (tiles 
        relative to themself that they could perform an attack against on their turn). Some enemies may not attack every turn.

## Interface

All of the game takes place on a single command prompt or terminal window. 
Various windows will be displayed on this terminal window during gameplay, which are the following:

- In the centre is the main game window.
- In the top left are basic statistics, including current health, position, floor number, and turns completed.
- Below the previous window is a window displaying the extent of the player's current attacks.
- In the top right corner, current held keys are displayed.
- On the right of the main window, a log of attacks and major events is displayed, with timestamps.

### Controls

Movement can be achieved using any of the following key sets (choose whichever you are most comfortable with):

    - wasd
    - arrow keys
    - hjkl 

The player may also choose to do nothing for a turn, which is performed by pressing the period ('.').
To end the current run, press the escape key.
To return to the most recently used door, press 'r'. This can only be done when no enemies are on screen.

## Running The Game

The github repository contains, in the target/release/ folder, an optimised executable file that runs the game.
Alternatively, you can build the game from the source code and run the resulting binary. See [Building From Source](#Building-From-Source)

## Dependencies

Below are all the dependencies directly required for the game. You do not have to worry about installing them; they
are already included in the executable.
Even if you choose to build from source, Cargo automatically downloads dependencies as they are specified in the 
Cargo.toml file.

- [bandit][bandit]: Crate for traditional roguelike or other turn based and grid based games.
- [crossterm][crossterm]: Cross platform terminal manipulation.
- [point](https://github.com/That-H/point): Two dimensional co-ordinates used extensively in representing internal
objects.
- [rect](https://github.com/That-H/rect): Basic rectangle handling involved in procedural map generation.
- [windowed](https://github.com/That-H/windowed): Terminal windowing library used to display the main game window
as well as various stats windows simultaneously.

[bandit]: https://github.com/That-H/bandit
[crossterm]: https://crates.io/crates/crossterm

### Building From Source

Doing the following will reqiure both [git](https://git-scm.com/) and Cargo (the Rust build system) to be installed.

	git clone https://github.com/That-H/untitled_bandit
	cd untitled_bandit
	cargo build --release
	cargo run

These commands will create a local copy of the repository, cd into it, and run it with optimisation. Note that the build step
is required as there will already be an executable in the target/release folder in the local copy, which will be overwritten.
