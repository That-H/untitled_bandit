# 0.9.3-alpha
Some puzzle solution optimisations and health visibility.

## New Features
- Pressing the letter c allows the player to see the current health of all enemies.
- A license.

# 0.9.2-beta
Adds some finishing touches.

## New Content
- Added 5 bonus puzzles.

## General Changes
- Made the attack of the n more clear.
- Moved one advanced puzzle to intermediate.

# 0.9.2-alpha
Major content update.

## New Features
- Made the player regain two health after each floor.

## New Content
- Added a fifth floor with a small questline to reach it. 
- Added a K and V.
- Added c and n.
- New Puzzles
    - +2 Intermediate
    - +2 Advanced
    - +1 Extreme

## Balance Changes
- Moved various bosses to account for the new floor.
    - R floor range 2-2 -> 3-3
    - B floor range 2-2 -> 3-3
    - O floor range 1-1 -> 2-2
- Stopped a few basic enemies from spawning on floor 1.
    - e floor range 0-1 -> 0-0
    - h floor range 0-1 -> 0-0
    - o floor range 0-1 -> 0-0
- Lowered the cost of various enemies
- Made the b unable to attack if it doesn't move first.
- Made the q wait every third turn to make floor 3 more fair.

## General Changes
- Made ice now appear as cyan asterisks instead of asterisks on a cyan background, which was quite harsh on the eyes.
- Changed the rng used internally to xoshiro256++ for portability purposes.
- Changed the enemy budget formula to be (area of room + FLOOR\_NUM * 30)
- Made enemies unable to spawn in the 5x5 boxes centred on each door.
- No longer uses total turns for score calculation.
- Killing enemies is rewarded more the higher the floor number is.
- Added a screen between floors informing the player of some statistics of that floor and giving them the option to give up.
- Changed the colours of floors 2 and 3 to dark yellow with green doors and blue gray.

## Performance
- Vastly improved gameplay performance by only drawing characters that actually changed since the last frame.

# 0.9.1-alpha
Content update with some slight alphabet visual changes.

## New Content
- Letters a, i, s and x.
- Boss L, which fires missiles.
- New puzzles:
    - +2 beginner
    - +2 intermediate
    - +3 advanced
    - +1 extreme

## General Changes
- Made the enemy budget for a room dependent on the number of free tiles within the room instead of the 
area of its bounding rect.
- Made the colour of the info box in the alphabet dependent on the range of floors on which the enemy spawns.
- Made the '.' in the info box disappear if it is highlighted.
- Changed the message that appears in the alphabet screen when no enemies have been killed so that 
it is more helpful (it is now "Come back when you've killed more enemies...")

## Balance Changes
- Moved the Q as it is not very difficult to kill.
    - Floor range 1-1 -> 0-0

# 0.9.0
Cuts corners and adds the alphabet.

## New Features
- Adds the alphabet to give the player information on each enemy they have killed.
- Cuts corners off of some rooms to make map generation more interesting.

## Regression Fixes
- Fixed some moves on ice being counted multiple times.

## New Content
- 16 new puzzles
- 2 new normal enemies 
- 2 new boss enemies

# 0.8.3-beta
Fixes some visual bugs introduced by the alphabet.

## New Content
- New puzzles:
    - +1 intermediate
    - +1 advanced

## General Changes
- Made the puzzle selection window larger.
- Added a title to the puzzles screen.
- Added an indicator to the puzzle screen that displays how many stars have been collected
in total. Also a completion progress percentage.

## Regression Fixes
- Fixed remnants of the end screen still being visible after going to the main menu.
- Fixed the title's edges being visible when opening the alphabet.

# 0.8.3-alpha2
Alphabetical!

## New Features
- The game now also keeps track of how many of each enemy you have killed.
- Alphabet option on the main menu. This gives the player information on every enemy
they have killed at least once.

## Regression Fixes
- Fixed ice puzzles sometimes connecting to the void when corners are cut near them.

# 0.8.3-alpha
Corner cutting.

## New Content
- New puzzles:
    - +1 advanced

## New Features
- Sometimes snips the corners off of rooms. Also removes some other nearby walls if it does so. Will
not cur corners in ice rooms. This feature is highly unstable.
- Added an indicator during puzzles as to how many stars have currently been earned on this puzzle.

## General Changes
- Modified the doors to use a floodfill algorithm to reveal the room instead of using the entire bounding 
rect of the room.
- Stopped counting damage dealt to doors. This fixes an exploit where a player could artificially increase
combat efficiency by attacking doors continuously.
- Slightly lowered amount of walls appearing in ice rooms (probability 0.3 -> 0.25).
- Changed the position of the exit tile in the boss room to be the first valid position diagonally down and
right from the top left corner of the room.

# 0.8.2-beta
Score improvements.

## New Content
- New puzzles:
    - +1 intermediate
    - +2 advanced

## General Changes
- Prevented any instance of NaN appearing by replacing it with 0.
- Modified the scoring formula to reward killing to a greater extent.
- Changed end screen outline colour from white to grey.

## Balance Changes
- R is pretty violent, so it has been swapped with Q (which is on a similar level to O).
    - R floor range 1-1 -> 2-2
    - Q floor range 2-2 -> 1-1

## Regression Fixes
- Prevented enemies dealing damage counting towards the total damage dealt.

# 0.8.2-alpha3
Adds scoring and other stats.

## New Features
- New end screen statistics:
    - Damage dealt over the course of the run (not visible if a puzzle was played)
    - Combat efficiency (amount of damage dealt divided by time in combat)
    - Score (based on enemies killed, floor reached, combat efficiency, turns taken)
- High score saving

## New Content
- New puzzles:
- Moved puzzles:
    - 2 advanced -> intermediate

## Balance Changes
- Slightly changed the order in which the letter g tries each adjacent tile to make it
less oppressive.
- Moved R to floor 1 as it is noticeably more difficult than E.

# 0.8.2-alpha2
Hotfix for the last hotfix that turned into a minor content update.

## New Features 
- New puzzles:
    - +1 beginner
    - +2 intermediate
    - +2 advanced
- Moved puzzles:
    - 1 advanced -> intermediate
- Letter q enemy.
- Letter Q boss.

## Regression Fixes
- Fixed a crash caused by trying to play the last puzzle.

## General Changes
- Made the appearance of diagonal attacks slightly more prominent.
- Upon clearing a puzzle, the number of moves of the best known solution is displayed.

## Balance Changes
- Moved R to floor 0.
- Rebalanced r as it isn't as powerful as previously thought.
    - Floor range 2-3 -> 1-2
    - Cost 35 -> 27

## Compatability
- Automatically resizes the terminal at the start of the game.
- Leaves the terminal in a more normal state after quitting (disables raw mode, clears the screen,
moves the cursor to the top left corner).

# 0.8.2-alpha
Extra enemies and puzzles.

## New Features
- Letter r enemy.
- Letter R boss.
- +1 intermediate puzzle.
- +3 advanced puzzles.
- +1 extreme puzzle.

## Regression Fixes
- Fixed the mismatch between the puzzle number displayed while playing a puzzle and when selecting one.

# 0.8.1
Hotfix for death/win screen menu.

## Regression Fixes
- Made the window on the death/win screen large enough to fit the save and quit button.

# 0.8.0
Large update adding puzzles with progress saving.

## New Features 
- 15 Puzzles ranging in difficulty from beginner to extreme.
- A save and quit button that saves progress with puzzles.
- A scrolling puzzle selection menu.

## Regression Fixes
- Fixed maps occasionally being generated that are impossible to complete.

# 0.7.4-beta2
Internal improvements.

## General Changes 
- Made a more informative error message appear if the puzzles file is unable to be read for some reason.
- Made the retry button appear at the top of the menu if the player dies during a puzzle. Otherwise 'next puzzle'
is the top option.

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
- Added the O boss to floor 1. It attacks any tile exactly two king moves away.
- Added the letter g, which moves and attacks like a king. Waits every third turn.
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
