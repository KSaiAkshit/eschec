import chess
import chess.engine
import argparse
import chess.pgn
import random
import os

PGN_DATABASE_PATH = "lichess_db_standard_rated_2014-10.pgn"
OUTPUT_EPD_PATH = "balanced_book.epd"
STOCKFISH_PATH = "/usr/games/stockfish"

EVAL_TIME_LIMIT = 0.1
EVAL_SCORE_THRESHOLD = 50

MIN_PLY = 16
MAX_PLY = 40
GAMES_TO_PROCESS = 200000
TARGET_POSITIONS = 1000
OPENING_ID_PLY = 12


def create_book(pgn_path, output_path, stockfish_path, limit, target_positions):
    """
    Parses a PGN file, finds positions with near equal eval using Stockfish,
    and saves them to an EPD file for engine testing
    """
    print("Starting opening book generation...")
    print(f"  - PGN Source: {pgn_path}")
    print(f"  - Output EPD: {output_path}")
    print(f"  - Uniqueness Ply: {OPENING_ID_PLY}")
    print(f"  - Stockfish: {stockfish_path}")

    if not os.path.exists(stockfish_path):
        print(f"Error: Stockish engine not found at '{stockfish_path}'")
        return

    if not os.path.exists(pgn_path):
        print(f"Error: PGN database not found at '{pgn_path}'")
        return

    if not os.path.exists(pgn_path):
        print(f"Info: Output file not found at '{output_path}'")
        return

    try:
        engine = chess.engine.SimpleEngine.popen_uci(stockfish_path)
    except Exception as e:
        print(f"Failed to start Stockfish engine: {e}")
        return

    positions_found = 0
    games_processed = 0
    used_opening_fingerprints = set()

    with open(pgn_path) as pgn, open(output_path, "w") as epd_file:
        while True:
            if games_processed >= limit or positions_found >= target_positions:
                break

            try:
                game = chess.pgn.read_game(pgn)
            except (ValueError, RuntimeError) as e:
                print(f"Skipping a malformed game: {e}")
                continue

            if game is None:
                print("Reached end of PGN file.")
                break

            games_processed += 1

            if games_processed % 1000 == 0:
                print(
                    f"Processed {games_processed} / {limit} games..\n Found {positions_found} / {target_positions} positions"
                )

            board = game.board()
            moves = list(game.mainline_moves())

            if len(moves) < MAX_PLY:
                continue

            fingerprint_board = game.board()
            for i in range(OPENING_ID_PLY):
                fingerprint_board.push(moves[i])

            opening_fingerprint = fingerprint_board.fen()
            if opening_fingerprint in used_opening_fingerprints:
                continue

            ply_to_play = random.randrange(MIN_PLY, MAX_PLY + 1)

            for i in range(ply_to_play):
                board.push(moves[i])

            try:
                if (
                    board.is_checkmate()
                    or board.is_stalemate()
                    or board.is_insufficient_material()
                ):
                    continue

                info = engine.analyse(board, chess.engine.Limit(time=EVAL_TIME_LIMIT))
                score_obj = info["score"].white()
                score = score_obj.score(mate_score=20000)

                if score is not None and abs(score) <= EVAL_SCORE_THRESHOLD:
                    fen = board.fen()
                    epd_line = f'{fen} id "{positions_found + 1}"; ce {score}\n'
                    print(f"Found: {epd_line}")
                    epd_file.write(epd_line)
                    positions_found += 1
            except (
                chess.engine.EngineError,
                chess.engine.EngineTerminatedError,
                TypeError,
            ) as e:
                print(f"An engine error occured: {e}, Restarting engine...")
                engine.quit()
                engine = chess.engine.SimpleEngine.popen_uci(stockfish_path)

    print("------------------------------")
    print(f"Finished! found: {positions_found} balanced positions.")
    print(f"Book saved to {output_path}")
    engine.quit()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Create a chess opening book of balanced positions using Stockfish."
    )
    parser.add_argument(
        "--pgn", default=PGN_DATABASE_PATH, help="Path to the PGN database file."
    )
    parser.add_argument(
        "--output", default=OUTPUT_EPD_PATH, help="Path for the output EPD file."
    )
    parser.add_argument(
        "--stockfish", default=STOCKFISH_PATH, help="Path to the Stockfish executable."
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=GAMES_TO_PROCESS,
        help="Max number of games to process from the PGN.",
    )
    parser.add_argument(
        "--target",
        type=int,
        default=TARGET_POSITIONS,
        help="Number of balanced positions to find.",
    )
    args = parser.parse_args()

    create_book(args.pgn, args.output, args.stockfish, args.limit, args.target)
