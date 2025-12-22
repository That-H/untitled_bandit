# v0.6.3-beta

## New Features
- Main menu
- Death screen allowing the player to quit or go back to the main menu.
- Win screen
- Added the omega boss ('Ω') to floor 3.

## Balance Changes
- The g enemy is too powerful to only appear on floor 2.
    - Floor range 2-2 -> 2-3
- The v is simple enough to appear on floor 2 as well.
    - Floor range 3-3 -> 2-3
- Lowercase b is often a difficult enemy to kill, so its cost has increased.
    - Cost 34 -> 48

## General Changes
- Made the attack of the O show all the tiles it can attack.

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
