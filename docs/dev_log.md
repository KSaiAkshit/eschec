# Dev Log

* Date: 2025-08-18
* New: tag 'v0.1.7-late_move_red'
* Old: tag 'v0.1.6-aspiration_win'

### fixed depth late move reduction (reduction = 1)

#### 8moves_v3.pgn
Results of eschec vs eschec_aspiration_win (15+0.1, NULL, NULL, 8moves_v3.pgn):
Elo: 77.71 +/- 33.03, nElo: 117.50 +/- 48.15
LOS: 100.00 %, DrawRatio: 47.00 %, PairsRatio: 3.42
Games: 200, Wins: 67, Losses: 23, Draws: 110, Points: 122.0 (61.00 %)
Ptnml(0-2): [1, 11, 47, 25, 16], WL/DD Ratio: 0.27
LLR: 1.71 (57.9%) (-2.94, 2.94) [0.00, 10.00]

#### balanced_book.epd
Results of eschec vs eschec_aspiration_win (15+0.1, NULL, NULL, balanced_book.epd):
Elo: 123.02 +/- 40.58, nElo: 159.46 +/- 48.15
LOS: 100.00 %, DrawRatio: 25.00 %, PairsRatio: 3.69
Games: 200, Wins: 102, Losses: 34, Draws: 64, Points: 134.0 (67.00 %)
Ptnml(0-2): [1, 15, 25, 33, 26], WL/DD Ratio: 2.12
LLR: 2.19 (74.4%) (-2.94, 2.94) [0.00, 10.00]


---

* Date: 2025-08-16
* New: tag 'v0.1.6-asipration_win'
* Old: tag 'v0.1.5-king_safetyv2'

### stricter bounds, asymmetric widening, "centered" on prev score

#### 8moves_v3.pgn
Results of eschec vs eschec_king_safetyv2 (15+0.1, NULL, NULL, 8moves_v3.pgn):
Elo: 26.11 +/- 31.23, nElo: 40.51 +/- 48.15
LOS: 95.04 %, DrawRatio: 42.00 %, PairsRatio: 1.32
Games: 200, Wins: 45, Losses: 30, Draws: 125, Points: 107.5 (53.75 %)
Ptnml(0-2): [1, 24, 42, 25, 8], WL/DD Ratio: 0.11
LLR: 0.60 (20.3%) (-2.94, 2.94) [0.00, 10.00]

#### balanced_book.epd
Results of eschec vs eschec_king_safetyv2 (15+0.1, NULL, NULL, balanced_book.epd):
Elo: 41.89 +/- 33.28, nElo: 61.39 +/- 48.15
LOS: 99.38 %, DrawRatio: 32.00 %, PairsRatio: 1.72
Games: 200, Wins: 63, Losses: 39, Draws: 98, Points: 112.0 (56.00 %)
Ptnml(0-2): [2, 23, 32, 35, 8], WL/DD Ratio: 0.60
LLR: 0.92 (31.1%) (-2.94, 2.94) [0.00, 10.00]

#### 2moves.pgn
Results of eschec vs eschec_king_safetyv2 (15+0.1, NULL, NULL, 2moves.pgn):
Elo: 34.86 +/- 80.64, nElo: 47.72 +/- 107.67
LOS: 80.75 %, DrawRatio: 35.00 %, PairsRatio: 1.17
Games: 40, Wins: 10, Losses: 6, Draws: 24, Points: 22.0 (55.00 %)
Ptnml(0-2): [0, 6, 7, 4, 3], WL/DD Ratio: 0.00
LLR: 0.15 (5.1%) (-2.94, 2.94) [0.00, 10.00]


### stricter bounds, symmetric widening, centered on current best score
Results of eschec vs eschec_king_safetyv2 (15+0.1, NULL, NULL, 2moves.pgn):
Elo: 70.44 +/- 69.23, nElo: 114.24 +/- 107.67
LOS: 98.12 %, DrawRatio: 55.00 %, PairsRatio: 3.50
Games: 40, Wins: 10, Losses: 2, Draws: 28, Points: 24.0 (60.00 %)
Ptnml(0-2): [0, 2, 11, 4, 3], WL/DD Ratio: 0.00
LLR: 0.35 (11.9%) (-2.94, 2.94) [0.00, 10.00]

### stricter bounds, symmetric widening, centered on prev score
Results of eschec vs eschec_king_safetyv2 (15+0.1, NULL, NULL, 2moves.pgn):
Elo: 70.44 +/- 82.58, nElo: 96.36 +/- 107.67
LOS: 96.03 %, DrawRatio: 40.00 %, PairsRatio: 3.00
Games: 40, Wins: 12, Losses: 4, Draws: 24, Points: 24.0 (60.00 %)
Ptnml(0-2): [1, 2, 8, 6, 3], WL/DD Ratio: 0.00
LLR: 0.27 (9.2%) (-2.94, 2.94) [0.00, 10.00]

### symmetric widening, centered on prev score
Results of eschec vs eschec_king_safetyv2 (15+0.1, NULL, NULL, 2moves.pgn):
Elo: 8.69 +/- 66.67, nElo: 14.21 +/- 107.67
LOS: 60.20 %, DrawRatio: 40.00 %, PairsRatio: 1.00
Games: 40, Wins: 12, Losses: 11, Draws: 17, Points: 20.5 (51.25 %)
Ptnml(0-2): [0, 6, 8, 5, 1], WL/DD Ratio: 1.67
LLR: 0.03 (1.0%) (-2.94, 2.94) [0.00, 10.00]


---

* Date: 2025-08-15
* New: tag 'v0.1.5-king_safetyv2'
* Old: tag 'v0.1.4-pawn_masks'

### Introduced more penalties & bonuses for king safety
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
