# Dev Log

* Date: 2025-08-15
* New: tag 'v0.1.5-king_safetyv2'
* Old: tag 'v0.1.4-pawn_masks'

Results of eschec vs eschec_pawn_masks (15+0.1, NULL, NULL, 2moves.pgn):
Elo: 10.43 +/- 17.99, nElo: 19.76 +/- 34.05
LOS: 87.23 %, DrawRatio: 53.00 %, PairsRatio: 1.19
Games: 400, Wins: 67, Losses: 55, Draws: 278, Points: 206.0 (51.50 %)
Ptnml(0-2): [1, 42, 106, 46, 5], WL/DD Ratio: 0.12
LLR: 0.49 (16.7%) (-2.94, 2.94) [0.00, 10.00]

---

* Date: 2025-08-09
* New: tag 'v0.1.4-pawn_masks'
* Old: tag 'v0.1.3-repetition_detection'

### Faster & appropriate pawn evaluation
Results of eschec_pawn_masks vs eschec_repetition_detection (15+0.1, NULL, NULL, 2moves.pgn):
Elo: 52.51 +/- 43.47, nElo: 83.99 +/- 68.10
LOS: 99.22 %, DrawRatio: 50.00 %, PairsRatio: 3.17
Games: 100, Wins: 25, Losses: 10, Draws: 65, Points: 57.5 (57.50 %)
Ptnml(0-2): [2, 4, 25, 15, 4], WL/DD Ratio: 0.09
LLR: 0.58 (19.8%) (-2.94, 2.94) [0.00, 10.00]

---

* Date: 2025-08-08
* New: tag 'v0.1.3-repetition_detection'
* Old: tag 'v0.1.2-prng_mo'

### Threefold repetition detection and ply adjustment
Results of eschec_repetition_detection vs eschec_prng_mo (15+0.1, NULL, NULL, 2moves.pgn):
Elo: 66.82 +/- 33.08, nElo: 141.45 +/- 68.10
LOS: 100.00 %, DrawRatio: 42.00 %, PairsRatio: 4.80
Games: 100, Wins: 25, Losses: 6, Draws: 69, Points: 59.5 (59.50 %)
Ptnml(0-2): [0, 5, 21, 24, 0], WL/DD Ratio: 0.05
LLR: 0.85 (28.9%) (-2.94, 2.94) [0.00, 10.00]

---

* Date: 2025-08-08
* New: tag 'v0.1.2-prng_mo'
* Old: tag 'v0.1.1-persistent_tt'

### PRNG based tie-breaking in move-ordering
Results of eschec_prng_mo vs eschec_persistent_tt (15+0.1, NULL, NULL, 2moves.pgn):
Elo: 209.91 +/- 30.05, nElo: 603.02 +/- 68.10
LOS: 100.00 %, DrawRatio: 6.00 %, PairsRatio: inf
Games: 100, Wins: 54, Losses: 0, Draws: 46, Points: 77.0 (77.00 %)
Ptnml(0-2): [0, 0, 3, 40, 7], WL/DD Ratio: 0.00
LLR: 1.45 (49.4%) (-2.94, 2.94) [0.00, 10.00]

---

* Date: 2025-08-07
* New: tag 'v0.1.1-persistent_tt'
* Old: tag 'v0.1.0'

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

---
