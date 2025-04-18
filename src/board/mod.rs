use evaluation::Evaluator;
use miette::Context;
use rand::prelude::*;
use std::{collections::HashMap, fmt::Display};
use tracing::*;

use moves::Moves;

use self::components::{BoardState, CastlingRights, Piece, Side, Square};

pub mod components;
pub mod evaluation;
mod fen;
pub mod moves;
pub mod search;

/// Completely encapsulate the game
#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Board {
    // Snapshot of current board
    pub positions: BoardState,
    /// Side to move, 0 - white, 1 - black
    pub stm: Side,
    /// Castling rights for both sides, KQ - White king&queen side, kq - Black king&queen side, '-' no right
    pub castling_rights: CastlingRights,
    /// En passant target square
    pub enpassant_square: Option<Square>,
    /// Specifies a number of half-moves with respect to the 50 move draw rule. It is reset(to 0) after a capture
    /// or a pawn move and incremented otherwise.
    pub halfmove_clock: u8,
    ///  The number of the full moves in a game. It starts at 1, and is incremented after each Black's move.
    pub fullmove_counter: u8,
    /// Material left for each side [White, Black]
    pub material: [u64; 2],
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Create top border with file labels
        writeln!(f, "  +---+---+---+---+---+---+---+---+")?;

        // Iterate through ranks (from top to bottom)
        for rank in (0..8).rev() {
            // Write rank number
            write!(f, "{} |", rank + 1)?;

            // Iterate through files (from left to right)
            for file in 0..8 {
                let square_idx = rank * 8 + file;

                // Check if there's a piece at this square
                let mut piece_found = false;

                for (piece_type, side) in Piece::all() {
                    let side_idx = side.index();
                    let piece_idx = piece_type.index();

                    if self.positions.all_pieces[side_idx][piece_idx].contains_square(square_idx) {
                        // Draw the piece using Unicode chess piece
                        write!(f, " {} |", piece_type.icon(side))?;
                        piece_found = true;
                        break;
                    }
                }

                // If no piece found, draw empty square
                if !piece_found {
                    // Use different background for alternating squares
                    let is_dark = (rank + file) % 2 == 1;
                    if is_dark {
                        write!(f, " · |")?;
                    } else {
                        write!(f, "   |")?;
                    }
                }
            }

            // End of rank
            writeln!(f)?;
            writeln!(f, "  +---+---+---+---+---+---+---+---+")?;
        }

        // File letters at the bottom
        writeln!(f, "    A   B   C   D   E   F   G   H  ")?;

        // Additional game information
        writeln!(f, "\nSide to move: {}", self.stm)?;
        writeln!(f, "Castling rights: {}", self.castling_rights)?;

        if let Some(ep) = self.enpassant_square {
            writeln!(f, "En passant square: {}", ep)?;
        } else {
            writeln!(f, "En passant square: -")?;
        }

        writeln!(f, "Halfmove clock: {}", self.halfmove_clock)?;
        writeln!(f, "Fullmove counter: {}", self.fullmove_counter)?;
        writeln!(
            f,
            "Material balance: W:{} B:{}",
            self.material[Side::White.index()],
            self.material[Side::Black.index()]
        )?;

        Ok(())
    }
}

impl Board {
    /// Use to initialize a default board
    pub fn new() -> Self {
        const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        // const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let mut board = Self {
            positions: BoardState::default(),
            stm: Side::default(),
            castling_rights: CastlingRights::default(),
            enpassant_square: Option::default(),
            halfmove_clock: u8::default(),
            fullmove_counter: u8::default(),
            material: [u64::default(); 2],
        };
        match board.place_pieces(START_FEN) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error initializing board: {}", e);
            }
        }
        board
    }
    /// Use this to construct a board from fen
    pub fn from_fen(fen: &str) -> Self {
        let parsed = fen::parse_fen(fen);
        match parsed {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Got error while parsing given fen: {}", e);
                panic!("very bad fen")
            }
        }
    }

    pub fn generate_legal_moves(&self) -> miette::Result<Vec<(Square, Square)>> {
        // NOTE: Is this correct as well?
        let mut legal_moves = Vec::with_capacity(40);
        let side_index = self.stm.index();
        let our_pieces = self.positions.all_sides[side_index];

        let king_pos = self.positions.all_pieces[side_index][Piece::king()]
            .iter_bits()
            .next()
            .wrap_err("King should be alive and gettable")?;
        let _king_square = Square::new(king_pos)
            .wrap_err_with(|| format!("king_pos {king_pos} should be valid"))?;
        for piece_type in Piece::colored_pieces(self.stm) {
            let piece_bb = self.positions.all_pieces[side_index][piece_type.index()];

            for from_idx in piece_bb.iter_bits() {
                let from_sq = Square::new(from_idx)
                    .wrap_err_with(|| format!("king_pos {from_idx} should be valid"))?;

                let moves = Moves::new(piece_type, self.stm, self.positions);
                let potential_moves = moves.attack_bb[from_idx] & !our_pieces;

                for to_idx in potential_moves.iter_bits() {
                    let to_sq = Square::new(to_idx)
                        .wrap_err_with(|| format!("king_pos {to_idx} should be valid"))?;
                    let mut b_copy = *self;
                    b_copy.try_move(from_sq, to_sq);
                    if !b_copy.is_in_check(b_copy.stm) {
                        legal_moves.push((from_sq, to_sq));
                    }
                }
            }
        }

        Ok(legal_moves)
    }

    pub fn make_move(&mut self, from: Square, to: Square) -> miette::Result<()> {
        if !self.is_move_legal(from, to) {
            miette::bail!("Illegal move from {} to {}", from, to);
        }
        if let Some(piece) = self.get_piece_at(from) {
            self.positions
                .update_piece_position(&piece, &self.stm, from, to);

            self.handle_special_rules(from, to);
            self.calculate_material();
            self.stm = self.stm.flip();
            self.halfmove_clock += 1;
            if self.stm == Side::White {
                self.fullmove_counter += 1;
            }
            Ok(())
        } else {
            miette::bail!("No piece at from Square");
        }
    }

    pub fn is_move_legal(&self, from: Square, to: Square) -> bool {
        // NOTE: Is this correct? seems like something is missing
        // Check if there is a piece at the 'from' square
        let piece = match self.get_piece_at(from) {
            Some(p) => p,
            None => return false,
        };
        let state = self.positions;

        // Check if the piece belongs to the current side to move
        if !self.positions.all_sides[self.stm.index()].contains_square(from.index()) {
            return false;
        }

        // Generate legal moves for the piece
        let mut moves = Moves::new(piece, self.stm, state);
        moves.make_legal(&self.stm, &self.positions);
        let legal_squares = moves.attack_bb[from.index()];

        // Check if the 'to' square is a legal square for the piece
        if !legal_squares.contains_square(to.index()) {
            return false;
        }

        // Check if the move puts own king in check
        let mut board = *self;
        board.try_move(from, to);
        !board.is_in_check(self.stm)
    }

    // To be used on a copy of the board
    fn try_move(&mut self, from: Square, to: Square) {
        if let Some(piece) = self.get_piece_at(from) {
            self.positions
                .update_piece_position(&piece, &self.stm, from, to);

            self.handle_special_rules(from, to);
            self.calculate_material();
            self.halfmove_clock += 1;
            if self.stm == Side::White {
                self.fullmove_counter += 1;
            }
        }
    }

    pub fn is_in_check(&self, side: Side) -> bool {
        // 1. Find king's position
        let king_bb = self.positions.all_pieces[side.index()][Piece::King.index()];
        let king_square =
            Square::new(king_bb.get_set_bits()[0]).expect("Should be able to find king");

        // 2. Generate oppponent's attacks
        let opponent = side.flip();
        let state = self.positions;
        for piece in Piece::colored_pieces(opponent) {
            let piece_bb = self.positions.all_pieces[opponent.index()][piece.index()];
            let moves = Moves::new(piece, self.stm, state);
            // If any opponent piece can attack king's square, king is in check
            if piece_bb
                .get_set_bits()
                .iter()
                .any(|&from| moves.attack_bb[from].contains_square(king_square.index()))
            {
                return true;
            }
        }

        false
    }

    pub fn is_checkmate(&self, side: Side) -> bool {
        self.is_in_check(side)
            && !self
                .generate_legal_moves()
                .expect("Should be able to gen legal moves")
                .is_empty()
    }

    pub fn is_stalemate(&self, side: Side) -> bool {
        !self.is_in_check(side)
            && self
                .generate_legal_moves()
                .expect("Should be able to gen legal moves")
                .is_empty()
    }

    pub fn is_draw(&self) -> bool {
        self.is_stalemate(self.stm) || self.halfmove_clock >= 100 || self.is_insufficient_material()
    }

    pub fn suggest_rand_move(&self) -> miette::Result<(Square, Square)> {
        let mut rng = rand::rng();
        let mut possible_end_bits: Vec<usize> = Vec::default();
        let mut from = Square::default();
        let mut to = Square::default();
        while possible_end_bits.is_empty() {
            // Choose a piece at random
            let (piece, _) = Piece::all()
                .choose(&mut rng)
                .expect("Should be able to choose at random");
            // Generate moves for the randomly selected piece
            let mut moves = Moves::new(piece, self.stm, self.positions);
            moves.make_legal(&self.stm, &self.positions);

            // Get the position of the Piece on the current board
            let piece_state = self.positions.all_pieces[self.stm.index()][piece.index()];
            let piece_idx = piece_state.get_set_bits();

            let piece_choice = piece_idx
                .choose(&mut rng)
                .expect("Should be able to get a random piece idx");
            let m = moves.attack_bb[*piece_choice];
            possible_end_bits = m.get_set_bits();
            if possible_end_bits.is_empty() {
                continue;
            }
            from = Square::new(*piece_choice).expect("Should be valid piece choice");
            let end_bit = possible_end_bits
                .choose(&mut rng)
                .expect("Should be able to get random to square");
            to = Square::new(*end_bit).expect("Should be valid square");
        }
        Ok((from, to))
    }

    pub fn evaluate_position(&self, evaluator: &dyn Evaluator) -> i32 {
        evaluator.evaluate(self)
    }

    // NOTE: Older Implementation without support for full length FEN strings
    fn place_pieces(&mut self, fen: &str) -> miette::Result<()> {
        if fen.contains(' ') {
            return Err(miette::Error::msg("Not supported for now"));
        }
        let lookup_table: HashMap<char, (Piece, Side)> = [
            ('P', (Piece::Pawn, Side::White)),
            ('p', (Piece::Pawn, Side::Black)),
            ('B', (Piece::Bishop, Side::White)),
            ('b', (Piece::Bishop, Side::Black)),
            ('N', (Piece::Knight, Side::White)),
            ('n', (Piece::Knight, Side::Black)),
            ('R', (Piece::Rook, Side::White)),
            ('r', (Piece::Rook, Side::Black)),
            ('Q', (Piece::Queen, Side::White)),
            ('q', (Piece::Queen, Side::Black)),
            ('K', (Piece::King, Side::White)),
            ('k', (Piece::King, Side::Black)),
        ]
        .into_iter()
        .collect();
        // rank [7,0]
        let mut rank = 7;
        // file [0,7]
        let mut file = 0;
        // dbg!(rank, file);
        for c in fen.chars() {
            // dbg!("---------------------");
            // dbg!(c);
            match c {
                '1'..='8' => {
                    // dbg!(rank, file);
                    file += c.to_digit(10).unwrap() as usize;
                    // dbg!(rank, file);
                }
                '/' => {
                    // dbg!(rank, file);
                    rank -= 1;
                    file = 0;
                    // dbg!(rank, file);
                }
                _ => {
                    if let Some((piece, side)) = lookup_table.get(&c) {
                        // dbg!(piece, side);
                        // dbg!(rank, file);
                        self.positions.all_pieces[side.index()][piece.index()].set(rank * 8 + file);
                        file += 1;
                    } else {
                        miette::bail!("Invalid Fen Character")
                    }
                }
            }
        }
        // updte all_sides too
        self.positions.all_sides[0] = self.positions.all_pieces[0][0]
            | self.positions.all_pieces[0][1]
            | self.positions.all_pieces[0][2]
            | self.positions.all_pieces[0][3]
            | self.positions.all_pieces[0][4]
            | self.positions.all_pieces[0][5];

        self.positions.all_sides[1] = self.positions.all_pieces[1][0]
            | self.positions.all_pieces[1][1]
            | self.positions.all_pieces[1][2]
            | self.positions.all_pieces[1][3]
            | self.positions.all_pieces[1][4]
            | self.positions.all_pieces[1][5];
        Ok(())
    }

    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        let index = square.index();

        for (piece_type, side) in Piece::all() {
            let piece_bb = self.positions.all_pieces[side.index()][piece_type.index()];
            if piece_bb.contains_square(index) {
                return Some(piece_type);
            }
        }
        None
    }

    pub fn handle_special_rules(&mut self, from: Square, to: Square) {
        // Handle castling
        if let Some(Piece::King) = self.get_piece_at(from) {
            // Update castling rights
            match self.stm {
                Side::White => self
                    .castling_rights
                    .remove_right(CastlingRights::WHITE_CASTLING),
                Side::Black => self
                    .castling_rights
                    .remove_right(CastlingRights::BLACK_CASTLING),
            }
            // Move rook if castling
            // TODO: Implement castling logic
        }

        // Handle en passant
        if let Some(Piece::Pawn) = self.get_piece_at(from) {
            // Set en passant square if double pawn push
            // A double pawn push moves two squares forward from starting position
            let is_double_push = match self.stm {
                Side::White => from.index() / 8 == 1 && to.index() / 8 == 3,
                Side::Black => from.index() / 8 == 6 && to.index() / 8 == 4,
            };

            // If double push detected, set en passant square to the skipped square
            if is_double_push {
                let skipped_square = Square::new(match self.stm {
                    Side::White => from.index() + 8,
                    Side::Black => from.index() - 8,
                })
                .unwrap();
                self.enpassant_square = Some(skipped_square);
            }

            // If moving to en passant square, capture the enemy pawn
            if let Some(ep_square) = self.enpassant_square {
                if to == ep_square {
                    let captured_pawn_square = Square::new(match self.stm {
                        Side::White => ep_square.index() - 8,
                        Side::Black => ep_square.index() + 8,
                    })
                    .unwrap();
                    self.positions.all_pieces[self.stm.flip().index()][Piece::Pawn.index()]
                        .capture(captured_pawn_square.index());
                }
            }
        }
    }

    fn calculate_material(&mut self) {
        // Reset material counts
        self.material = [0; 2];

        for side in [Side::White, Side::Black] {
            let side_index = side.index();
            for piece in Piece::colored_pieces(side) {
                let piece_bb = self.positions.all_pieces[side_index][piece.index()];
                let piece_count = piece_bb.0.count_ones() as u64;
                let piece_value: u64 = u32::from(piece) as u64;
                self.material[side_index] += piece_count * piece_value;
            }
        }
    }

    // fn is_insufficient_material(&self) -> bool {
    //     let white_pieces = self.positions.all_sides[Side::White.index()];
    //     let black_pieces = self.positions.all_sides[Side::Black.index()];

    //     let mut white_counts = [0; 6];
    //     let mut black_counts = [0; 6];

    //     for piece in Piece::PIECES.iter() {
    //         white_counts[piece.index()] = self.positions.all_pieces[Side::White.index()]
    //             [piece.index()]
    //         .get_set_bits()
    //         .len();
    //         black_counts[piece.index()] = self.positions.all_pieces[Side::Black.index()]
    //             [piece.index()]
    //         .get_set_bits()
    //         .len();
    //     }

    //     if white_pieces.0.count_ones() == 1 && black_pieces.0.count_ones() == 1 {
    //         return true;
    //     }
    //     // King and Bishop vs King
    //     if (white_counts[Piece::King.index()] == 1
    //         && white_counts[Piece::Bishop.index()] == 1
    //         && white_pieces.0.count_ones() == 2
    //         && black_counts[Piece::King.index()] == 1
    //         && black_pieces.0.count_ones() == 1)
    //         || (black_counts[Piece::King.index()] == 1
    //             && black_counts[Piece::Bishop.index()] == 1
    //             && black_pieces.0.count_ones() == 2
    //             && white_counts[Piece::King.index()] == 1
    //             && white_pieces.0.count_ones() == 1)
    //     {
    //         return true;
    //     }

    //     // King and Knight vs King
    //     if (white_counts[Piece::King.index()] == 1
    //         && white_counts[Piece::Knight.index()] == 1
    //         && white_pieces.0.count_ones() == 2
    //         && black_counts[Piece::King.index()] == 1
    //         && black_pieces.0.count_ones() == 1)
    //         || (black_counts[Piece::King.index()] == 1
    //             && black_counts[Piece::Knight.index()] == 1
    //             && black_pieces.0.count_ones() == 2
    //             && white_counts[Piece::King.index()] == 1
    //             && white_pieces.0.count_ones() == 1)
    //     {
    //         return true;
    //     }

    //     // King and Bishop vs King and Bishop (same colored squares)
    //     if white_counts[Piece::King.index()] == 1
    //         && white_counts[Piece::Bishop.index()] == 1
    //         && black_counts[Piece::King.index()] == 1
    //         && black_counts[Piece::Bishop.index()] == 1
    //         && self.same_colored_bishops()
    //     {
    //         return true;
    //     }

    //     false
    // }

    fn is_insufficient_material(&self) -> bool {
        let white_pieces = self.positions.all_sides[Side::White.index()];
        let black_pieces = self.positions.all_sides[Side::Black.index()];

        // Arrays to store the counts of each piece type
        let mut white_counts = [0; 6];
        let mut black_counts = [0; 6];

        // Count the pieces for both sides
        for piece in Piece::PIECES.iter() {
            white_counts[piece.index()] =
                self.positions.all_pieces[Side::White.index()][piece.index()].pop_count();

            black_counts[piece.index()] =
                self.positions.all_pieces[Side::Black.index()][piece.index()].pop_count();
        }

        // If both sides have only their kings, it's insufficient material
        if white_pieces.0.count_ones() == 1 && black_pieces.0.count_ones() == 1 {
            return true;
        }

        // King and Bishop vs King (White or Black can have King and Bishop)
        if (white_counts[Piece::King.index()] == 1
            && white_counts[Piece::Bishop.index()] == 1
            && white_pieces.0.count_ones() == 2
            && black_counts[Piece::King.index()] == 1
            && black_pieces.0.count_ones() == 1)
            || (black_counts[Piece::King.index()] == 1
                && black_counts[Piece::Bishop.index()] == 1
                && black_pieces.0.count_ones() == 2
                && white_counts[Piece::King.index()] == 1
                && white_pieces.0.count_ones() == 1)
        {
            return true;
        }

        // King and Knight vs King (White or Black can have King and Knight)
        if (white_counts[Piece::King.index()] == 1
            && white_counts[Piece::Knight.index()] == 1
            && white_pieces.0.count_ones() == 2
            && black_counts[Piece::King.index()] == 1
            && black_pieces.0.count_ones() == 1)
            || (black_counts[Piece::King.index()] == 1
                && black_counts[Piece::Knight.index()] == 1
                && black_pieces.0.count_ones() == 2
                && white_counts[Piece::King.index()] == 1
                && white_pieces.0.count_ones() == 1)
        {
            return true;
        }

        // King and Bishop vs King and Bishop (same colored squares)
        if white_counts[Piece::King.index()] == 1
            && white_counts[Piece::Bishop.index()] == 1
            && black_counts[Piece::King.index()] == 1
            && black_counts[Piece::Bishop.index()] == 1
            && self.same_colored_bishops()
        {
            return true;
        }

        // Additional check for opposite colored bishops (same as insufficient material)
        if white_counts[Piece::King.index()] == 1
            && white_counts[Piece::Bishop.index()] == 1
            && black_counts[Piece::King.index()] == 1
            && black_counts[Piece::Bishop.index()] == 1
            && !self.same_colored_bishops()
        {
            return true;
        }

        // If none of the above conditions are met, it's not insufficient material
        false
    }

    fn same_colored_bishops(&self) -> bool {
        let white_bishop = self.positions.all_pieces[Side::White.index()][Piece::Bishop.index()];
        let black_bishop = self.positions.all_pieces[Side::Black.index()][Piece::Bishop.index()];

        if let (Some(white_sq), Some(black_sq)) = (
            white_bishop.get_set_bits().first(),
            black_bishop.get_set_bits().first(),
        ) {
            // Bishops are on same colored squares if sum of their coordinates is even/odd same
            let (white_file, white_rank) = Square::new(*white_sq).unwrap().coords();
            let (black_file, black_rank) = Square::new(*black_sq).unwrap().coords();

            let a = (white_file + white_rank) % 2;
            let b = (black_file + black_rank) % 2;
            a == b
        } else {
            false
        }
    }

    #[instrument(skip(self, evaluator), fields(eval_name = evaluator.name()))]
    pub fn find_best_move(
        &self,
        nodes_searched: &mut u64,
        evaluator: &dyn Evaluator,
        depth: u8,
    ) -> miette::Result<(Square, Square)> {
        info!(side = %self.stm, "Finding best move for");

        info!("getting legal moves");
        let legal_moves = self.generate_legal_moves()?;
        if legal_moves.is_empty() {
            miette::bail!("No legal moves available")
        }
        info!(legal_moves.num = legal_moves.len(), "got legal moves: ");

        let mut best_score = i32::MIN;
        let mut best_move = legal_moves[0];
        info!(
            best_score = best_score,
            from = %best_move.0,
            to = %best_move.1,
            "init vals"
        );

        for (from, to) in legal_moves {
            let mut board_copy = *self;
            board_copy.make_move(from, to)?;
            info!(
                best_score = best_score,
                from = %from,
                to = %to,
                depth = depth,
                "currently on move"
            );

            let score = -self.minimax(
                &board_copy,
                nodes_searched,
                depth - 1,
                i32::MIN,
                i32::MAX,
                false,
                evaluator,
            );

            if score > best_score {
                best_score = score;
                best_move = (from, to);
            }
        }
        Ok(best_move)
    }

    #[instrument(skip(self, board, evaluator))]
    fn minimax(
        &self,
        board: &Board,
        mut nodes_searched: &mut u64,
        depth: u8,
        mut alpha: i32,
        mut beta: i32,
        maximizing_player: bool,
        evaluator: &dyn Evaluator,
    ) -> i32 {
        trace!("staring minimax search");
        *nodes_searched += 1;
        if depth == 0 || board.is_draw() || board.is_checkmate(board.stm) {
            warn!("got deep");
            return board.evaluate_position(evaluator);
        }
        let legal_moves = match board.generate_legal_moves() {
            Ok(moves) => moves,
            Err(_) => return board.evaluate_position(evaluator),
        };
        if maximizing_player {
            let mut max_eval = i32::MIN;
            for (from, to) in legal_moves {
                let mut board_copy = *board;
                if board_copy.make_move(from, to).is_err() {
                    continue;
                }

                let eval = self.minimax(
                    &board_copy,
                    &mut nodes_searched,
                    depth - 1,
                    alpha,
                    beta,
                    false,
                    evaluator,
                );
                max_eval = max_eval.max(eval);
                alpha = alpha.max(eval);
                if beta <= alpha {
                    break;
                }
            }
            return max_eval;
        } else {
            let mut min_eval = i32::MAX;
            for (from, to) in legal_moves {
                let mut board_copy = *board;
                if board_copy.make_move(from, to).is_err() {
                    continue;
                }

                let eval = self.minimax(
                    &board_copy,
                    &mut nodes_searched,
                    depth - 1,
                    alpha,
                    beta,
                    true,
                    evaluator,
                );
                min_eval = min_eval.min(eval);
                beta = beta.min(eval);
                if beta <= alpha {
                    break;
                }
            }
            return min_eval;
        }
    }
}

#[cfg(test)]
mod material_tests {

    use super::*;

    #[test]
    fn test_initial_material_balance() {
        let mut board = Board::new();
        board.calculate_material();
        // Initial material for each side should be:
        // 8 pawns (8 * 100 = 800)
        // 2 knights (2 * 320 = 640)
        // 2 bishops (2 * 330 = 660)
        // 2 rooks (2 * 500 = 1000)
        // 1 queen (1 * 900 = 900)
        // 1 king (1 * 20000 = 20000)
        // Total: 24000
        assert_eq!(board.material[Side::White.index()], 24000);
        assert_eq!(board.material[Side::Black.index()], 24000);
    }

    #[test]
    fn test_material_after_capture() {
        let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKBNR w KQkq - 0 1");
        board.calculate_material();
        // Standard position with a missing f2 pawn on white's side

        assert_eq!(board.material[Side::White.index()], 23900); // 24000 - 100 = 23900
        assert_eq!(board.material[Side::Black.index()], 24000);
    }

    #[test]
    fn test_king_vs_king() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_king_and_bishop_vs_king() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KB2 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_king_and_knight_vs_king() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KN2 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_kings_and_same_colored_bishops() {
        // Bishops on the same color squares (both on light squares)
        let board = Board::from_fen("2b1k3/8/8/8/8/8/8/2B1K3 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_kings_and_different_colored_bishops() {
        // Bishops on different color squares (one on light, one on dark)
        let board = Board::from_fen("1b2k3/8/8/8/8/8/8/2B1K3 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_sufficient_material() {
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4KB2 w - - 0 1");

        assert!(!board.is_insufficient_material());
    }

    #[test]
    fn test_two_knights_sufficient_material() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KNN1 w - - 0 1");

        assert!(!board.is_insufficient_material());
    }

    #[test]
    fn test_two_bishops_sufficient_material() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/3BKB2 w - - 0 1");

        assert!(!board.is_insufficient_material());
    }
}
