import chess.pgn
import random
import math

pgn = open("lichess_db_standard_rated_2014-10.pgn")
positions = []

for i in range(150000):
    game = chess.pgn.read_game(pgn)
    if game is not None:
        moves = game.mainline_moves()
        board = game.board()
        fen = str()

        plyToPlay = math.floor(16 + 20 * random.random()) & ~1
        numPlyPlayed = 0
        for move in moves:
            board.push(move)
            numPlyPlayed += 1
            if numPlyPlayed >= plyToPlay:
                fen = board.fen()

        numPiecesInPos = sum(fen.lower().count(char) for char in "rhbq")
        if numPlyPlayed > plyToPlay + 20 * 2 and numPiecesInPos >= 10:
            positions.append(game.headers["Opening"])
            positions.append(fen)
            print(f"found pos: {fen}, {int( positions.__len__() / 2 )} / 150000")


with open("output.txt") as file:
    for string in positions:
        file.write(string + "\n")
