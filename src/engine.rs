extern crate rust_chess;

#[allow(dead_code)]
pub mod teros_engine {
    use std::{
        collections::{BinaryHeap, VecDeque},
        f32::{INFINITY, NEG_INFINITY},
    };

    use ordered_float::NotNan;
    use std::cmp::Ordering;
    use std::collections::BTreeMap;

    use rust_chess::chess::chess::*;

    fn piece_worth(piece: PieceKind) -> NotNan<f32> {
        match piece {
            PieceKind::Pawn => NotNan::new(1.0).unwrap(),
            PieceKind::Knight => NotNan::new(3.0).unwrap(),
            PieceKind::Bishop => NotNan::new(3.0).unwrap(),
            PieceKind::Rook => NotNan::new(5.0).unwrap(),
            PieceKind::Queen => NotNan::new(9.0).unwrap(),
            PieceKind::King => NotNan::new(INFINITY).unwrap(),
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]

    struct ValuedMoveLocation {
        valued_move: ValuedChessMove,
        location: VecDeque<ChessMove>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
    struct ValuedChessMove {
        value: NotNan<f32>, // probably precise enough
        chess_move: ChessMove,
    }

    impl ValuedMoveLocation {
        fn value_accounted_for_distance(&self) -> NotNan<f32> {
            self.valued_move.value - self.location.len() as f32
        }
    }

    impl Ord for ValuedMoveLocation {
        fn cmp(&self, other: &Self) -> Ordering {
            match self
                .value_accounted_for_distance()
                .cmp(&other.value_accounted_for_distance())
            {
                Ordering::Equal => match self.valued_move.cmp(&other.valued_move) {
                    Ordering::Equal => match self.location.cmp(&other.location) {
                        Ordering::Equal => {
                            assert_eq!(self, other);
                            Ordering::Equal
                        }
                        Ordering::Greater => Ordering::Greater,
                        Ordering::Less => Ordering::Less,
                    },
                    Ordering::Greater => Ordering::Greater,
                    Ordering::Less => Ordering::Less,
                },
                Ordering::Greater => Ordering::Greater,
                Ordering::Less => Ordering::Less,
            }
        }
    }

    impl PartialOrd for ValuedMoveLocation {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
    struct MoveTree {
        board_state: Board,
        moves: BTreeMap<ChessMove, MoveTree>,
    }

    impl MoveTree {
        pub fn is_leaf(&self) -> bool {
            return self.moves.len() == 0;
        }

        pub fn print_tree(&self, depth: i32, max_depth: i32) {
            if depth > max_depth {
                return;
            }
            for (chess_move, tree) in self.moves.iter() {
                for _ in 0..depth {
                    print!("  |");
                }
                print!("-");
                println!("{}", chess_move.name(&self.board_state).unwrap());
                tree.print_tree(depth + 1, max_depth);
            }
        }
    }

    #[derive(Debug)]
    pub struct Engine {
        moves: BinaryHeap<ValuedMoveLocation>,
        move_tree: MoveTree,
    }

    #[derive(Clone, Debug)]
    pub enum EngineError {
        InvalidLocationError,
        NoValidMovesErrror,
        IllegalMoveError,
    }

    impl<'a> Engine {
        pub fn new(board: Board) -> Engine {
            let mut res = Engine {
                moves: BinaryHeap::new(),
                move_tree: MoveTree {
                    moves: BTreeMap::new(),
                    board_state: board,
                },
            };
            res.generate_all_moves(VecDeque::new()).unwrap();
            res
        }

        pub fn get_board(&'a self) -> &'a Board {
            &self.move_tree.board_state
        }

        //go to a branch specified by the list of moves in location.
        fn go_to_location(
            &'a mut self,
            location: &VecDeque<ChessMove>,
        ) -> Result<&'a mut MoveTree, EngineError> {
            let mut current_tree = &mut self.move_tree;
            for chess_move in location {
                match current_tree.moves.get_mut(chess_move) {
                    Some(child_tree) => {
                        current_tree = child_tree;
                    }
                    None => {
                        return Err(EngineError::InvalidLocationError);
                    }
                }
            }
            Ok(current_tree)
        }

        pub fn interpret_and_make_move(&mut self, move_string: &str) -> Result<(), EngineError> {
            let chess_move = self
                .move_tree
                .board_state
                .interpret_move(move_string)
                .ok()
                .ok_or(EngineError::IllegalMoveError)?;
            self.make_move(&chess_move)?;
            Ok(())
        }

        pub fn make_move(&mut self, chess_move: &ChessMove) -> Result<(), EngineError> {
            self.move_tree = self
                .move_tree
                .moves
                .get_mut(chess_move)
                .ok_or(EngineError::IllegalMoveError)?
                .clone();
            self.moves
                .retain(|x| x.location.len() > 0 && x.location[0] == *chess_move);

            let mut new_moves = self.moves.clone().into_vec();

            new_moves.iter_mut().for_each(|x| {
                x.location.pop_front();
            });

            self.moves = BinaryHeap::from(new_moves);

            if self.move_tree.moves.is_empty() {
                self.generate_all_moves(VecDeque::new()).unwrap();
            }

            Ok(())
        }

        fn generate_all_moves(&mut self, location: VecDeque<ChessMove>) -> Result<(), EngineError> {
            let tree = self.go_to_location(&location)?;
            for i in 0..BOARD_SIZE {
                for j in 0..BOARD_SIZE {
                    match tree.board_state.generate_moves(i, j) {
                        Ok(moves) => {
                            for chess_move in moves {
                                let mut new_board = tree.board_state.clone();
                                match new_board.make_legal_move(chess_move) {
                                    Ok(()) => {
                                        tree.moves.insert(
                                            chess_move,
                                            MoveTree {
                                                board_state: new_board,
                                                moves: BTreeMap::new(),
                                            },
                                        );
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                        Err(BoardError::NoPieceError | BoardError::WrongTurnError) => {}
                        Err(_) => {
                            panic!("what");
                        }
                    }
                }
            }
            let mut new_moves = Vec::new();

            for (chess_move, ending_board) in tree.moves.iter() {
                new_moves.push(ValuedMoveLocation {
                    valued_move: ValuedChessMove {
                        chess_move: chess_move.clone(),
                        value: Engine::evaluate_interest(
                            chess_move,
                            &tree.board_state,
                            &ending_board.board_state,
                        )
                        .unwrap(),
                    },
                    location: location.clone(),
                });
            }
            for new_move in new_moves {
                self.moves.push(new_move);
            }
            Ok(())
        }

        pub fn print_tree(&self, depth: i32) {
            self.move_tree.print_tree(0, depth);
        }

        pub fn print_moves(&mut self) {
            let mut temp_heap = self.moves.clone();

            while let Some(valued_move) = temp_heap.pop() {
                println!("{:#?}", valued_move);
            }
        }

        pub fn think_next_move(&mut self) -> Result<(), EngineError> {
            let next_move = self.moves.pop().ok_or(EngineError::NoValidMovesErrror)?;
            let mut location = next_move.location;
            location.push_back(next_move.valued_move.chess_move);
            self.generate_all_moves(location)?;
            Ok(())
        }

        fn evaluate_interest(
            chess_move: &ChessMove,
            starting_board: &Board,
            ending_board: &Board,
        ) -> Result<NotNan<f32>, BoardError> {
            use ChessMove::*;
            //temporary
            let res = match chess_move {
                Normal(normal_move) => Engine::evaluate_normal_move_interest(
                    normal_move,
                    starting_board,
                    ending_board,
                )?,
                Promotion(normal_move, piece_kind) => {
                    Engine::evaluate_normal_move_interest(
                        normal_move,
                        starting_board,
                        ending_board,
                    )? + piece_worth(*piece_kind)
                }
                Castling(_) => NotNan::new(5.0).unwrap(),
            };

            Ok(res)
        }

        fn evaluate_normal_move_interest(
            normal_move: &NormalChessMove,
            starting_board: &Board,
            ending_board: &Board,
        ) -> Result<NotNan<f32>, BoardError> {
            Ok(
                // match starting_board.get_piece(normal_move.initial_row, normal_move.initial_col)? {
                //     None => NotNan::new(0.0).unwrap(),
                //     Some(Piece {kind: PieceKind::Pawn, color }) => {
                //         let past = is_past_pawn(normal_move.destination_row, normal_move.destination_col, ending_board, color);
                //         match past {
                //             true => NotNan::new(normal_move.destination_row as f32).unwrap(),
                //             false => NotNan::new(0.0).unwrap()
                //         }
                //     }
                //     Some(_) => {
                //         NotNan::new(0.0).unwrap()
                //     }
                // } +
                match starting_board.get_piece(normal_move.destination_row, normal_move.destination_col)? {
                Some(piece) => {
                    piece_worth(piece.kind)
                }
                None => {
                    NotNan::new(0.0).unwrap()
                }
            } * 1.2//again pulled out of nowhere
             + match ending_board.is_check.is_some() {
                true => match ending_board.is_checkmate.is_some() {
                    true => INFINITY,
                    false => 5.0 //no reason no method
                }
                false => 0.0
            }
            + ((Engine::controlling_squares(ending_board, starting_board.get_turn())
             - Engine::controlling_squares(starting_board,starting_board.get_turn())
            ) as f32)*0.2,
            )
        }

        pub fn eval_and_best_move(&self) -> (NotNan<f32>, Option<ChessMove>) {
            Engine::minimax(&self.move_tree, 1000, self.move_tree.board_state.get_turn() == Color::White)
        }

        fn minimax(
            tree: &MoveTree,
            depth: i32,
            maximizing_player: bool,
        ) -> (NotNan<f32>, Option<ChessMove>) {
            if depth == 0 || tree.is_leaf() {
                return (Engine::static_evaluation(&tree.board_state), None);
            }

            if maximizing_player {
                let mut max_eval = NotNan::new(std::f32::NEG_INFINITY).unwrap();
                let mut best_move = None;
                for (chess_move, child) in tree.moves.clone() {
                    let eval = Engine::minimax(&child, depth - 1, false).0;
                    if eval > max_eval {
                        max_eval = eval;
                        best_move = Some(chess_move)
                    }
                }
                return (max_eval, best_move);
            } else {
                let mut min_eval = NotNan::new(std::f32::INFINITY).unwrap();
                let mut best_move = None;
                for (chess_move, child) in tree.moves.clone() {
                    let eval = Engine::minimax(&child, depth - 1, true).0;
                    if eval < min_eval {
                        min_eval = eval;
                        best_move = Some(chess_move)
                    }
                }
                return (min_eval, best_move);
            }
        }

        fn controlling_squares(board: &Board, color: Color) -> i32 {
            let mut squares = [[false; BOARD_SIZE]; BOARD_SIZE];
            for i in 0..BOARD_SIZE {
                for j in 0..BOARD_SIZE {
                    match board.get_piece(i, j).expect("Cant error always in bounds") {
                        Some(piece) => {
                            if piece.color != color {
                                continue;
                            }
                            for chess_move in board
                                .generate_moves_ignore_turn(i, j)
                                .expect("we know there's a piece there")
                            {
                                match chess_move {
                                    ChessMove::Normal(normal_move) => {
                                        squares[normal_move.destination_row]
                                            [normal_move.destination_col] = true;
                                    }
                                    ChessMove::Castling(_) => {}
                                    ChessMove::Promotion(normal_move, _) => {
                                        squares[normal_move.destination_row]
                                            [normal_move.destination_col] = true;
                                    }
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
            let count = squares.iter().flatten().filter(|&&x| x).count();
            count as i32
        }

        fn static_evaluation(board_state: &Board) -> NotNan<f32> {
            const SQUARE_CONTROL_WEIGHT: f32 = 0.2;
            const CHECK_WEIGHT: f32 = 3.0;
            match board_state.is_checkmate {
                None => {}
                Some(Color::White) => {
                    return NotNan::new(NEG_INFINITY).unwrap();
                }
                Some(Color::Black) => {
                    return NotNan::new(INFINITY).unwrap();
                }
            };
            let mut res = NotNan::new(0.0).unwrap();
            res += (Engine::controlling_squares(board_state, Color::White) as f32)
                * SQUARE_CONTROL_WEIGHT;
            res -= (Engine::controlling_squares(board_state, Color::Black) as f32)
                * SQUARE_CONTROL_WEIGHT;
            res += match board_state.is_check {
                Some(Color::Black) => CHECK_WEIGHT,
                None => 0.0,
                Some(Color::White) => -CHECK_WEIGHT,
            };
            for row in board_state.get_squares() {
                for piece_option in row {
                    res += match piece_option {
                        Some(piece) => {
                            (if piece.kind == PieceKind::King {
                                NotNan::new(0.0).unwrap()
                            }
                            else {
                                piece_worth(piece.kind)
                            }) * match piece.color {
                                Color::White => 1.0,
                                Color::Black => -1.0,
                            }
                        },
                        None => NotNan::new(0.0).unwrap(),
                    }
                }
            }
            res
        }
    }

    fn is_past_pawn(row: usize, col: usize, ending_board: &Board, color: Color) -> bool {
        let to_left_option = col.checked_sub(1);
        let to_center = col;
        let to_right = col + 1;
                        
        let cols = match to_left_option {
            Some(to_left) => vec![to_left, to_center, to_right],
            None => vec![to_center, to_right]
        };
                        
        let mut past = true;
        for i in row..BOARD_SIZE {
            for j in cols.clone() {
                let piece_result = ending_board.get_piece(i, j);
                match piece_result {
                Ok(Some(piece)) => {
                    if piece == (Piece{kind: PieceKind::Pawn, color: color.opposite()}) {
                    past = false;
                    }
                },
                _ => {}
                }
            }
    
        }
        past
    }
}
