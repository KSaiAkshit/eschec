# Dev Log

* Date: 2025-08-07 13:52
* Commit (old): 81f468dc7204a6cf298ef2a698c96ef51163a5db (feat: add option to change hash table size)
* Commit (new): 16b4b56793c73fac1136847210110f581783182c (feat: persistent tt btw searches)
### transient vs persistent tt (across searches in the same game)
Results of eschec_transient vs eschec_persistent (8+0.08, NULL, NULL, 2moves.pgn):
Elo: 70.44 +/- 53.06, nElo: 148.15 +/- 107.67
LOS: 99.65 %, DrawRatio: 40.00 %, PairsRatio: 5.00
Games: 40, Wins: 12, Losses: 4, Draws: 24, Points: 24.0 (60.00 %)
Ptnml(0-2): [0, 2, 8, 10, 0], WL/DD Ratio: 0.33
LLR: 0.35 (11.9%) (-2.94, 2.94) [0.00, 10.00]

### Setting appropriate depth and time
Results of eschec_transient vs eschec_persistent (8+0.08, NULL, NULL, 2moves.pgn):
Elo: 34.86 +/- 45.90, nElo: 52.38 +/- 68.10
LOS: 93.42 %, DrawRatio: 44.00 %, PairsRatio: 1.80
Games: 100, Wins: 26, Losses: 16, Draws: 58, Points: 55.0 (55.00 %)
Ptnml(0-2): [2, 8, 22, 14, 4], WL/DD Ratio: 0.22
LLR: 0.38 (12.9%) (-2.94, 2.94) [0.00, 10.00]

### Reset TT after every game
Results of eschec_transient vs eschec_persistent (8+0.08, NULL, NULL, 2moves.pgn):
Elo: -1.74 +/- 3.39, nElo: -24.69 +/- 48.15
LOS: 15.74 %, DrawRatio: 99.00 %, PairsRatio: 0.00
Games: 200, Wins: 0, Losses: 1, Draws: 199, Points: 99.5 (49.75 %)
Ptnml(0-2): [0, 1, 99, 0, 0], WL/DD Ratio: 0.00
LLR: -0.37 (-12.6%) (-2.94, 2.94) [0.00, 10.00]

----
