# 0.7.4-beta
Puzzle tweaks.

## New Features
- Modified some puzzles where better solutions were found.

## General Changes
- Comletion data about no longer existing puzzles is automatically deleted.

# 0.7.4-alpha3
Save game and more puzzles.

## New Features
- 4 Additional puzzles, one of which is extreme.
- Automatic saving of progress with puzzles.

# 0.7.4-alpha2
Scrolling menus and more puzzles.

## New Features
- Improved the puzzle selection screen.
    - Scrolling menu instead of static.
    - Dividers between blocks of puzzles based on subjective difficulty.
- Added five additional puzzles, one of which is advanced.

# 0.7.4-alpha
Puzzles.

## New Features
- Five puzzles accessible from the main menu for players to improve their efficiency.

# 0.7.3-alpha
MD5 hashing and a bug fix.

## New Features
- Added md5 hashing to allow seeds to consist of any characters, not just hexadecimal ones.

## Regression Fixes
- Fixed rooms occasionally generating in the void when the host room has no valid positions to generate from.

# 0.7.2-alpha4
Performance improvement and refactoring.

## New Features
- There is now an option allowing the player to input a seed for the run. This may only use hexadecimal digits.

## Performance
- Floor generation is now significantly faster.

# 0.7.2-alpha3
Cheats and regression fixes.

## New Features
- Added NoClip for debug builds.
- Added a seed tester that detects cases of previously seen regressions (e.g. impassable doors).

## Regression Fixes
- Fixed some doors being impossible to use.
- Fixed ice puzzles being unsolvable in rare cases.
- Fixed boss rooms occasionally teleporting the player to the next floor on entry.

## General Changes
- The key is now placed in the approximate centre of its room instead of randomly. This prevents potential confusion
from players picking up a key immediately upon the room.

# 0.7.2-alpha2
Adds some developer cheats and a bug fix.

## New Features
- Added a kill everyone cheat.
- Added a window displaying the current seed.

## Bug Fixes
- Fixed a regression making the key room occasionally overlap with an ice puzzle and cause the overlap section to be entirely doors.

# 0.7.2-alpha
Minor UI changes.

## New Features
- Menus with options that can be navigated between via arrow keys.

# 0.7.1
Hotfix allowing the project to be run when downloaded from github.

# 0.7.0
Adds an anti-softlock mechanism and all previous beta changes.

## New Features
- Returning to the previous door to prevent softlocks.

# v0.6.3-beta
Adds ice puzzles, end screens, and a new boss.

## New Features
- Main menu
- Death screen allowing the player to quit or go back to the main menu.
- Win screen
- Added the omega boss ('Ω') to floor 3.
- Rooms with more than one door have a 10% chance to become an ice puzzle room.

## Balance Changes
- The g enemy is too powerful to only appear on floor 2.
    - Floor range 2-2 -> 2-3
- The v is simple enough to appear on floor 2 as well.
    - Floor range 3-3 -> 2-3
- Lowercase b is often a difficult enemy to kill, so its cost has increased.
    - Cost 34 -> 48

## General Changes
- Made the attack of the O show all the tiles it can attack.
- Prevented the log displaying a health indicator if the target was a door.
- Reduced the size of the attack display window.

# v0.6.2-beta
This update mainly adds new enemies and a win condition.

## New Features
- Added the O boss to floor 1. It attacks any tile exactly two king moves away
- Added the letter g, which moves and attacks like a king. Waits every third turn
- Pseudo win screen (if you exit Floor 3, it says 'You win!')

## General Changes
- Swapped the colours of keys 3 and 4 to better fit the new environment they spawn in.

## Balance Changes
- The letter l seems too powerful to appear frequently and on floor 2, so it has been
rebalanced
    - Cost 45 -> 60
    - Floor range 2-3 -> 3-3

- The B boss is too complex to remain on floor 1.
    - Floor range 1-1 -> 2-2

# v0.6.1-beta
This update focuses on making individual floors unique by restricting enemies to certain floors and 
making the aesthetic change with each floor.

## New Features
- Made wall and floor colours cycle through dark grey, white, dark magenta and dark red
- Made door colours cycle through white, dark grey, orange and dark yellow
- Confined enemy types to specific floors
- Added a new E boss that spawns on floor 0
- Made enemy generation budget increase each floor

## General Changes
- Changed normal door colour to white

## Bug Fixes
- Fixed keys leaving behind an incorrectly coloured floor when picked up
