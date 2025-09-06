import chess
import sys


def parse_san_to_uci_file(file_content):
    """
    Parses the text file content into a structured list of dictionaries.

    Args:
        file_content (str): The content of the SAN to UCI text file.

    Returns:
        list: A list of dictionaries, where each dictionary represents a board state
              and its corresponding moves.
    """
    entries = []
    lines = file_content.strip().split("-----------------")

    for block in lines:
        block_lines = block.strip().split("\n")
        if not block_lines or not block_lines[0]:
            continue

        fen = block_lines[0].strip()

        # Skip the board ASCII art section if it exists
        if fen.startswith("+---"):
            # The FEN will be in the block after the ASCII art
            for i in range(len(block_lines)):
                if "Side to move:" in block_lines[i]:
                    fen = block_lines[i - 1].strip()
                    break

        moves = []
        for line in block_lines[1:]:
            line = line.strip()
            if line.startswith("Parsed SAN"):
                try:
                    parts = line.split(" -> ")
                    san_part = parts[0].split("'")[1]
                    # This line was updated to strip 'UCI '
                    uci_part = parts[1].strip().replace("'", "").replace("UCI ", "")

                    # Some SAN entries have an extra ' at the end of the string.
                    # e.g., 'a5\''
                    if san_part.endswith("'"):
                        san_part = san_part[:-1]

                    moves.append({"san": san_part, "uci": uci_part})
                except IndexError:
                    # Handle any malformed lines gracefully
                    print(f"Warning: Could not parse line: {line}")

        if fen and moves:
            entries.append({"fen": fen, "moves": moves})

    return entries


def verify_conversions(data):
    """
    Verifies SAN to UCI conversions for a given set of FEN and move data.

    Args:
        data (list): A list of dictionaries with 'fen' and 'moves' keys.
    """
    total_moves_checked = 0
    incorrect_moves_count = 0

    for entry in data:
        fen = entry["fen"]
        moves_to_verify = entry["moves"]

        try:
            board = chess.Board(fen)
            print(f"--- Verifying for FEN: {fen} ---")

            for move_data in moves_to_verify:
                san_move = move_data["san"]
                expected_uci = move_data["uci"]

                temp_board = board.copy()

                try:
                    move = temp_board.parse_san(san_move)
                    actual_uci = move.uci()

                    print(
                        f"SAN '{san_move}' -> Expected UCI: '{expected_uci}', Actual UCI: '{actual_uci}'",
                        end=" - ",
                    )
                    total_moves_checked += 1

                    if actual_uci == expected_uci:
                        print("Correct")
                    else:
                        print("Incorrect")
                        incorrect_moves_count += 1

                except (ValueError, IndexError) as e:
                    print(f"SAN '{san_move}' -> ERROR: {e}")
                    incorrect_moves_count += 1

            print("-" * 20)

        except ValueError as e:
            print(f"Failed to parse FEN: {fen} - ERROR: {e}")
            print("-" * 20)

    print(f"\nVerification complete.")
    print(f"Total moves checked: {total_moves_checked}")
    print(f"Total incorrect conversions: {incorrect_moves_count}")
    if incorrect_moves_count == 0:
        print("All moves are correct!")


def main(file_path):
    try:
        with open(file_path, "r") as file:
            file_content = file.read()

        data_to_verify = parse_san_to_uci_file(file_content)
        verify_conversions(data_to_verify)
    except FileNotFoundError:
        print(f"Error: The file '{file_path}' was not found.")
    except Exception as e:
        print(f"An error occurred: {e}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python san_to_uci_verifier.py <path_to_file>")
    else:
        file_path = sys.argv[1]
        main(file_path)
